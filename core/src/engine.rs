//! The input state machine. Pure logic: no Win32, no threads, no I/O, so it can
//! be driven directly from unit tests and from `--probe` without a window.
//!
//! Candidate layout (what the user asked for):
//!
//! ```text
//! nihao
//! 1    你好
//! 1.1  hallo        <- translation of candidate 1 (German)
//! 1.2  hello        <- translation of candidate 1 (English)
//! 1.3  こんにちは     <- translation of candidate 1 (Japanese)
//! 2    妳好
//! 2.1  hallo
//! ```
//!
//! The label *is* the key sequence: typing `1` narrows the list to candidate 1
//! and its translations, typing `1.1` selects the German translation, and so on.
//! Space/Enter commit the highlighted row, which is the deepest label typed so
//! far.

use std::collections::HashMap;

use crate::config::Config;
use crate::dict::Dict;
use crate::glossary::Glossary;
use crate::particle;
use crate::userdb::{now_secs, UserDb, LEARNED_BASE_SCORE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Space,
    Enter,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    /// 上一页（`-` / PageUp）
    PagePrev,
    /// 下一页（`+` / PageDown）
    PageNext,
}

/// What the caller should do with the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Not an IME key right now — let the application see it.
    Ignore,
    /// Swallowed; the UI state changed (repaint).
    Consumed,
    /// Swallowed; inject this text into the focused application.
    Commit(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// `1`, `1.1`, `1.2`, `2`, ...
    pub label: String,
    /// Text that gets committed when this row is chosen.
    pub text: String,
    /// 0 for a Chinese candidate, 1 for one of its translations.
    pub depth: u8,
    /// Language tag for translation rows (`德语`, `英文`, ...).
    pub lang: String,
}

#[derive(Clone)]
pub struct Engine {
    dict: Dict,
    glossary: Glossary,
    cfg: Config,

    ascii_mode: bool,
    composition: String,
    /// Digits and dots typed so far: `""`, `"1"`, `"1.1"`.
    /// 当前选中的组（页内 0 起）；None 表示还没按键选
    selected_group: Option<usize>,
    /// 选中层：0=中文（上层），1..N=第 N 条译文（下层可以有多条，连按数字循环）
    selected_tier: usize,
    /// Index into `rows` for arrow-key navigation.
    highlight: usize,
    rows: Vec<Row>,
    /// AI-provided translations merged in at runtime: word -> [(lang, text)].
    ai_cache: HashMap<String, Vec<(String, String)>>,
    /// Words we already asked the AI about, to avoid request storms.
    ai_requested: Vec<String>,
    /// 用户自学习：频率 + 联想 + 生僻词编码。
    userdb: UserDb,
    /// 上一个上屏的词，用于联想加权。
    last_committed: Option<String>,
    /// 引号开/关状态：`"` → “ 再按 → ”（和系统输入法一致）
    quote_double_open: bool,
    quote_single_open: bool,
    /// 当前候选页（每页 `page_size` 组，标签从 1 重新编号）
    page: usize,
    /// 候选池总数（分页前）
    pool_len: usize,
    /// 已经在问 AI 的拼音，避免每个键都发一次请求。
    spelling_requested: Vec<String>,
}

impl Engine {
    pub fn new(dict: Dict, glossary: Glossary, cfg: Config) -> Self {
        Self::with_userdb(dict, glossary, cfg, UserDb::default())
    }

    pub fn with_userdb(dict: Dict, glossary: Glossary, cfg: Config, userdb: UserDb) -> Self {
        Self {
            dict,
            glossary,
            ascii_mode: cfg.start_in_ascii_mode,
            cfg,
            composition: String::new(),
            selected_group: None,
            selected_tier: 0,
            highlight: 0,
            rows: Vec::new(),
            ai_cache: HashMap::new(),
            ai_requested: Vec::new(),
            userdb,
            last_committed: None,
            quote_double_open: true,
            quote_single_open: true,
            page: 0,
            pool_len: 0,
            spelling_requested: Vec::new(),
        }
    }

    pub fn userdb(&self) -> &UserDb {
        &self.userdb
    }

    pub fn userdb_mut(&mut self) -> &mut UserDb {
        &mut self.userdb
    }

    /// 换掉用户词库但保留它的文件绑定（重新加载词库时用）。
    pub fn replace_userdb(&mut self, mut userdb: UserDb) {
        if let Some(path) = self.userdb.path().map(|p| p.to_path_buf()) {
            userdb.bind(&path);
        }
        self.userdb = userdb;
    }

    /// 丢弃全部学习记录（自检/测试用，不写盘）。
    pub fn clear_userdb(&mut self) {
        self.userdb = UserDb::default();
        self.last_committed = None;
    }

    /// 落盘（调用方在主线程空闲时调用，比如每次上屏后或退出前）。
    pub fn flush_userdb(&mut self) -> std::io::Result<()> {
        self.userdb.save()
    }

    pub fn last_committed(&self) -> Option<&str> {
        self.last_committed.as_deref()
    }

    #[allow(dead_code)]
    pub fn composition(&self) -> &str {
        &self.composition
    }

    /// 当前选中的标签（`1` = 中文层，`1↓` = 英文层）；没选时返回空串。
    pub fn pending(&self) -> String {
        match self.selected_group {
            Some(group) => {
                if self.selected_tier == 0 {
                    (group + 1).to_string()
                } else {
                    format!("{}↓{}", group + 1, self.selected_tier)
                }
            }
            None => String::new(),
        }
    }

    /// 组 `group`（页内 0 起）的上层行号。
    fn parent_row(&self, group: usize) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.depth == 0 && row.label == (group + 1).to_string())
    }

    /// 组 `group` 的所有译文行号（按顺序）。
    fn child_rows(&self, group: usize) -> Vec<usize> {
        let Some(parent) = self.parent_row(group) else {
            return Vec::new();
        };
        self.rows
            .iter()
            .enumerate()
            .skip(parent + 1)
            .take_while(|(_, row)| row.depth == 1)
            .map(|(index, _)| index)
            .collect()
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn highlight(&self) -> usize {
        self.highlight
    }

    pub fn ascii_mode(&self) -> bool {
        self.ascii_mode
    }

    pub fn set_ascii_mode(&mut self, on: bool) {
        self.ascii_mode = on;
        if on {
            self.reset();
        }
    }

    pub fn toggle_ascii_mode(&mut self) {
        let next = !self.ascii_mode;
        self.set_ascii_mode(next);
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    #[allow(dead_code)]
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.cfg
    }

    /// 换一套配置并立刻按新配置重排候选（设置弹窗选完语言后调用）。
    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg;
        if !self.cfg.targets.is_empty() {
            // 换语言后旧的远端译文仍然可用；新语言会在下次请求时补上。
            self.ai_requested.clear();
        }
        if self.is_composing() {
            self.rebuild();
        }
    }

    /// True while a composition is visible (drives candidate window visibility).
    pub fn is_composing(&self) -> bool {
        !self.composition.is_empty()
    }

    pub fn dict_len(&self) -> usize {
        self.dict.len()
    }

    pub fn glossary_len(&self) -> usize {
        self.glossary.len()
    }

    /// Feed one key event. Mutates state and reports what to do with the key.
    pub fn handle(&mut self, key: Key) -> Outcome {
        if self.ascii_mode {
            return Outcome::Ignore;
        }
        match key {
            Key::Char(c) => self.handle_char(c),
            // 空格：上屏当前选中的候选层
            Key::Space => self.commit_or_passthrough(),
            // 回车：原样上屏拼音（和空格区分开，故意不选候选）
            Key::Enter => self.commit_raw_pinyin(),
            Key::Backspace => self.handle_backspace(),
            Key::Escape => {
                if self.is_composing() {
                    self.reset();
                    Outcome::Consumed
                } else {
                    Outcome::Ignore
                }
            }
            Key::Up => self.move_highlight(-1),
            Key::Down => self.move_highlight(1),
            Key::Left => self.move_highlight(-1),
            Key::Right => self.move_highlight(1),
            Key::PagePrev => self.flip_page(-1),
            Key::PageNext => self.flip_page(1),
        }
    }

    fn handle_char(&mut self, c: char) -> Outcome {
        if c.is_ascii_lowercase() {
            // A letter while a digit selection is pending means "commit what I
            // picked, then start typing the next word".
            if self.selected_group.is_some() {
                let text = self.selected_text().unwrap_or_default();
                self.record_learning(&text);
                self.reset();
                self.composition.push(c);
                self.rebuild();
                return if text.is_empty() {
                    Outcome::Consumed
                } else {
                    Outcome::Commit(text)
                };
            }
            self.composition.push(c);
            self.page = 0;
            self.rebuild();
            return Outcome::Consumed;
        }

        if c == '\'' || c == ' ' {
            // Syllable separator / commit key pressed as a character.
            return self.commit_or_passthrough();
        }

        if c.is_ascii_digit() {
            return self.handle_digit(c);
        }



        // `-` / `=` 在拼音输入状态下为翻页键（绝不能把未上屏候选配合 ＝ 误上屏）
        if (c == '-' || c == '=') && self.is_composing() {
            if self.page_count() > 1 {
                return self.flip_page(if c == '-' { -1 } else { 1 });
            }
            return Outcome::Consumed;
        }

        // 标点：正在输入时「先上屏候选再补标点」；没在输入时直接转中文标点
        // （微软拼音就是这个行为，不需要用户在候选里挑）。
        //
        // 例外：网址/英文串。`www` / `abc` 这种根本没有中文候选的输入，标点必须
        // 原样上屏，否则 `www.` 会变成 `www。`、`a.b` 变成 `a。b`。
        if let Some(mapped) = self.punctuation(c) {
            if self.is_composing() {
                let is_url_context = self.composition.starts_with("www")
                    || self.composition.starts_with("http")
                    || self
                        .last_committed
                        .as_deref()
                        .map_or(false, |p| p.ends_with('.') || p.starts_with("www") || p.starts_with("http"));
                if !self.composition_looks_like_pinyin() || (c == '.' && is_url_context) {
                    let text = self.composition.clone();
                    let committed = format!("{text}{c}");
                    self.record_learning(&committed);
                    self.reset();
                    return Outcome::Commit(committed);
                }
                let text = self.selected_text().unwrap_or_else(|| self.composition.clone());
                self.record_learning(&text);
                self.reset();
                return Outcome::Commit(format!("{text}{mapped}"));
            }
            return Outcome::Commit(mapped);
        }

        Outcome::Ignore
    }

    /// 数字键：先选中该组（中文层），**同一组再按一次切到英文层**，再按切回中文。
    /// 空格/回车提交当前高亮的那一层 —— 这就是「1 既能选中文也能选英文」。
    fn handle_digit(&mut self, c: char) -> Outcome {
        if !self.is_composing() {
            return Outcome::Ignore;
        }
        let Some(digit) = c.to_digit(10) else {
            return Outcome::Consumed;
        };
        let group = digit as usize - 1;
        if self.parent_row(group).is_none() {
            // 该页没有这个编号：吞掉，别把数字打进文档
            return Outcome::Consumed;
        }

        let tiers = self.child_rows(group).len();
        if self.selected_group == Some(group) {
            // 同一组连按：中文 → 第 1 条译文 → 第 2 条 … → 回到中文
            if tiers > 0 {
                self.selected_tier = (self.selected_tier + 1) % (tiers + 1);
            }
        } else {
            self.selected_group = Some(group);
            self.selected_tier = 0;
        }
        self.highlight = self.selected_row_index().unwrap_or(self.highlight);
        Outcome::Consumed
    }

    /// 当前选中层对应的行号。
    fn selected_row_index(&self) -> Option<usize> {
        let group = self.selected_group?;
        if self.selected_tier == 0 {
            return self.parent_row(group);
        }
        self.child_rows(group)
            .get(self.selected_tier - 1)
            .copied()
            .or_else(|| self.parent_row(group))
    }

    fn handle_backspace(&mut self) -> Outcome {
        if self.selected_group.is_some() {
            if self.selected_tier > 0 {
                self.selected_tier -= 1; // 先退层：译文 N -> 译文 N-1
            } else {
                self.selected_group = None; // 再退：取消选中
            }
            self.highlight = 0;
            return Outcome::Consumed;
        }
        if self.is_composing() {
            self.composition.pop();
            self.highlight = 0;
            if self.composition.is_empty() {
                self.reset();
            } else {
                self.rebuild();
            }
            return Outcome::Consumed;
        }
        Outcome::Ignore
    }

    /// 翻页：`-` / `+`（或 PageUp / PageDown）。翻页会清掉当前的层级选择。
    fn flip_page(&mut self, delta: isize) -> Outcome {
        if !self.is_composing() || self.page_count() <= 1 {
            return Outcome::Ignore;
        }
        let next = self.page as isize + delta;
        let last = self.page_count() as isize - 1;
        if next < 0 || next > last {
            return Outcome::Ignore;
        }
        self.page = next as usize;
        self.selected_group = None;
        self.selected_tier = 0;
        self.highlight = 0;
        self.rebuild();
        Outcome::Consumed
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn page_size(&self) -> usize {
        self.cfg.page_size.max(1)
    }

    /// 总页数（候选池按 `page_size` 分组）。
    pub fn page_count(&self) -> usize {
        let size = self.page_size();
        let len = self.pool_len;
        if len == 0 {
            1
        } else {
            (len + size - 1) / size
        }
    }

    fn move_highlight(&mut self, delta: isize) -> Outcome {
        if self.rows.is_empty() {
            return Outcome::Ignore;
        }
        let len = self.rows.len() as isize;
        let next = (self.highlight as isize + delta).clamp(0, len - 1);
        self.highlight = next as usize;
        // 方向键接管高亮：清掉数字选中的「组 + 层」状态
        self.selected_group = None;
        self.selected_tier = 0;
        Outcome::Consumed
    }

    /// 回车：把敲进去的拼音原样上屏（`bukeyi` → `bukeyi`），不做候选转换。
    fn commit_raw_pinyin(&mut self) -> Outcome {
        if !self.is_composing() {
            return Outcome::Ignore;
        }
        let raw = self.composition.clone();
        self.reset();
        Outcome::Commit(raw)
    }

    fn commit_or_passthrough(&mut self) -> Outcome {
        if !self.is_composing() {
            return Outcome::Ignore;
        }
        // No candidate at all: hand the raw pinyin to the document so unknown
        // input is never silently lost.
        let text = self
            .selected_text()
            .unwrap_or_else(|| self.composition.clone());
        self.record_learning(&text);
        self.reset();
        Outcome::Commit(text)
    }

    fn selected_text(&self) -> Option<String> {
        if self.rows.is_empty() {
            return None;
        }
        if let Some(index) = self.selected_row_index() {
            return self.rows.get(index).map(|row| row.text.clone());
        }
        self.rows.get(self.highlight).map(|r| r.text.clone())
    }

    fn reset(&mut self) {
        self.composition.clear();
        self.selected_group = None;
        self.selected_tier = 0;
        self.highlight = 0;
        self.page = 0;
        self.pool_len = 0;
        self.rows.clear();
    }

    /// 鼠标点选：按 `rows()` 里的下标提交某一层（父项=中文，子项=英文）。
    ///
    /// 返回要上屏的文本；越界或当前没有候选时返回 None。
    pub fn commit_row(&mut self, index: usize) -> Option<String> {
        let text = self.rows.get(index)?.text.clone();
        self.record_learning(&text);
        self.reset();
        Some(text)
    }

    /// 抛弃当前这次输入（焦点切换、失焦时用），不产生任何上屏。
    pub fn abort(&mut self) {
        self.reset();
    }

    /// 只读判断「这个键会不会被我们吃掉」，用于 TSF 的 OnTestKeyDown：
    /// 那里必须廉价且无副作用（有些应用会连发多次 OnTestKeyDown）。
    pub fn would_handle(&self, key: Key) -> bool {
        if self.ascii_mode {
            return false;
        }
        match key {
            Key::Char(c) if c.is_ascii_lowercase() => true,
            Key::Char(c) if c.is_ascii_digit() => self.is_composing(),
            Key::Char(c) if c == '.' => self.is_composing(),
            Key::Char(c) => self.is_composing() && chinese_punctuation(c).is_some(),
            Key::Space | Key::Enter | Key::Backspace | Key::Escape => self.is_composing(),
            Key::Up | Key::Down | Key::Left | Key::Right => !self.rows.is_empty(),
            // 翻页键只在有多页时才吞掉，否则让 `-` `+` 正常输入
            Key::PagePrev | Key::PageNext => self.is_composing() && self.page_count() > 1,
        }
    }

    /// Rebuild the flattened candidate list from the current composition.
    ///
    /// 候选池 = 词库匹配（精确/前缀）+ 用户自学习词（生僻词自动编码），
    /// 再用 `UserDb::score` 统一打分排序：词频越高、与上一个词共现越多，越靠前。
    fn rebuild(&mut self) {
        self.rows.clear();
        if self.composition.is_empty() {
            return;
        }

        let now = now_secs();
        let prev = self.last_committed.clone();
        let matches =
            self.dict
                .lookup(&self.composition, self.cfg.max_parents * 3, self.cfg.prefix_search);

        let mut pool: Vec<(String, i64)> = Vec::with_capacity(matches.len() + 8);
        for m in &matches {
            if pool.iter().any(|(word, _)| *word == m.word) {
                continue;
            }
            let score = self.userdb.score(m.rank, &m.word, prev.as_deref(), now);
            pool.push((m.word.clone(), score));
        }
        for word in self.userdb.learned(&self.composition) {
            if pool.iter().any(|(existing, _)| existing == word) {
                continue;
            }
            let score = self
                .userdb
                .score(LEARNED_BASE_SCORE as u32, word, prev.as_deref(), now);
            pool.push((word.clone(), score));
        }

        // 网络常用词与网址协议
        if self.composition == "www" {
            pool.push(("www.".to_string(), -200_000_000));
            pool.push(("www".to_string(), -190_000_000));
            pool.push(("https://www.".to_string(), -180_000_000));
            pool.push(("http://www.".to_string(), -170_000_000));
        } else if self.composition == "http" {
            pool.push(("http://".to_string(), -200_000_000));
            pool.push(("http".to_string(), -190_000_000));
        } else if self.composition == "https" {
            pool.push(("https://".to_string(), -200_000_000));
            pool.push(("https".to_string(), -190_000_000));
        }

        // 拼音一个候选都没有、但打的是英文词 → 反查中文（hello → 你好/喂）。
        // 只在「拼音完全没料」时启用，避免和正常拼音输入互相打断。
        let looks_latin = self.composition.len() >= 2
            && self
                .composition
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '\'');
        // 这次输入是否是词库中的精确拼音音节/词。只有非精确拼音时才启用英文反查，
        // 彻底避免像 deng（等/灯/登/邓）这样常用拼音被反查词（邓小平、邓世昌等）霸屏冲掉。
        let is_exact_pinyin = looks_latin && !self.dict.lookup(&self.composition, 1, false).is_empty();
        if looks_latin && (!is_exact_pinyin || pool.is_empty()) {
            for (word, rank, weight) in self
                .glossary
                .reverse_lookup(&self.composition, self.cfg.max_parents)
            {
                // weight 是该英文词在中文词释义中的次序，越大越能代表主意（pos 0 为 3，pos 1 为 2）
                let score = self
                    .userdb
                    .score(rank, &word, prev.as_deref(), now)
                    - (weight as i64) * 10_000
                    - 100_000_000;
                if let Some(slot) = pool.iter_mut().find(|(existing, _)| *existing == word) {
                    if score < slot.1 {
                        slot.1 = score;
                    }
                    continue;
                }
                pool.push((word, score));
            }

            // 英文联想词模式：当输入为非精确拼音或词库中缺少有效拼音候选时，推荐英文联想词候选
            if !is_exact_pinyin || pool.is_empty() {
                let assocs = self
                    .glossary
                    .english_associations(&self.composition, self.cfg.max_parents);
                for (order, assoc) in assocs.into_iter().enumerate() {
                    let is_exact = assoc.word.eq_ignore_ascii_case(&self.composition);
                    let score = if is_exact {
                        -100_015_000 // 精确匹配紧随主选反查中文词之后
                    } else {
                        -70_000_000 + (order as i64) * 10_000 + (assoc.rank as i64)
                    };
                    if let Some(slot) = pool
                        .iter_mut()
                        .find(|(existing, _)| existing.eq_ignore_ascii_case(&assoc.word))
                    {
                        if score < slot.1 {
                            slot.1 = score;
                        }
                    } else {
                        pool.push((assoc.word, score));
                    }
                }
            }
        }

        // 长串拆解（组句）：`bukeyitihuan` → 「不可以替换」+「不可以/替换」等拆法。
        // 这条解决了「连续 3~6 字拼音没有候选」的老问题。
        if looks_latin && self.composition.len() >= 4 {
            for seg in crate::segment::segment(&self.dict, &self.composition, 4) {
                if seg.words.len() < 2 {
                    continue; // 整串单词交给精确匹配
                }
                if pool.iter().any(|(existing, _)| *existing == seg.text) {
                    continue;
                }
                let score = (seg.cost * 3000.0) as i64;
                pool.push((seg.text, score));
            }
        }

        pool.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.len().cmp(&b.0.len())));
        pool.truncate(self.cfg.max_parents);
        self.pool_len = pool.len();
        if pool.is_empty() {
            // 没有任何候选（英文输入中途、生僻拼音）：放一条「原样拼音」行，
            // 保证候选窗不会突然消失；回车/空格都能把它原样上屏。
            // pool_len 仍保持 0，这样 AI 兜底查询（pending_spelling_query）会继续触发。
            let raw = self.composition.clone();
            self.rows.push(Row {
                label: "1".to_string(),
                text: raw,
                depth: 0,
                lang: "原样".to_string(),
            });
            if let Some(index) = self.selected_row_index() {
                self.highlight = index;
            }
            return;
        }

        // 每页 `page_size` 组；标签在页内从 1 重新编号（1..7），跨页用 +/- 翻。
        let size = self.page_size();
        let pages = self.page_count();
        if self.page >= pages {
            self.page = pages - 1;
        }
        let start = self.page * size;
        let slice = &pool[start..(start + size).min(pool.len())];

        for (idx, (word, _score)) in slice.iter().enumerate() {
            let label = (idx + 1).to_string();
            let mut children = self.children_for(word);
            if looks_latin && !is_exact_pinyin {
                // 打的是英文（`hello`）：下面再挂一行 `英文 hello` 毫无意义，
                // 那是用户刚打的词。反查模式下只保留中文候选本身。
                children.retain(|(lang, text)| !lang.contains('英') && !text.eq_ignore_ascii_case(&self.composition));
            }
            if children.is_empty() && !(looks_latin && !is_exact_pinyin) {
                // 组句候选（不可以替换这种）整句没有词典释义，用各词译文拼一条
                if let Some(joined) = self.composed_translation(word) {
                    children.push(("英文（拼合）".to_string(), joined));
                }
            }
            self.rows.push(Row {
                label,
                text: word.clone(),
                depth: 0,
                lang: String::new(),
            });
            for (child_idx, (lang, text)) in children.into_iter().enumerate() {
                self.rows.push(Row {
                    label: format!("{}.{}", idx + 1, child_idx + 1),
                    text,
                    depth: 1,
                    lang,
                });
            }
        }

        // 选中哪一层就把高亮放到哪一行（候选列表不再收缩，保持 7 组可见）。
        // 没有显式选择时，高亮必须回到第一组的**中文**行——译文永远不该是默认选中项。
        if let Some(index) = self.selected_row_index() {
            self.highlight = index;
        } else {
            self.highlight = 0;
        }
    }

    /// 给「拼出来的长句」现拼一条译文：`不可以替换` → `cannot replace`。
    ///
    /// 只在该短语整条查不到译文时使用；用的是分词结果里每个词的第一条英文，
    /// 任何一段缺译文就放弃（宁可没有，也不要半截胡话）。
    /// 刻意用空格而不是 ` / ` 拼接：读得通就像一句，读不通也会被后台免费网页
    /// 翻译补上的真正整句译文盖过去。
    fn composed_translation(&self, phrase: &str) -> Option<String> {
        let parts = crate::segment::segment(&self.dict, &self.composition, 1);
        let segmentation = parts.first()?;
        if segmentation.text != phrase || segmentation.words.len() < 2 {
            return None;
        }
        let english_only = ["en".to_string()];
        let mut pieces = Vec::with_capacity(segmentation.words.len());
        for word in &segmentation.words {
            let translated = self.glossary.translations(word, &english_only, 1);
            let text = translated.first()?.text.clone();
            pieces.push(text);
        }
        Some(pieces.join(" "))
    }

    /// Translations for one candidate:
    /// - For network words (e.g. www., http://): return descriptive translations
    /// - For English words (e.g. apple, computer): return Chinese translations ("中文")
    /// - For Chinese words: local glossary first, particle analysis for modal particles,
    ///   then remote AI / web translation cache.
    fn children_for(&self, word: &str) -> Vec<(String, String)> {
        if self.cfg.max_children == 0 {
            return Vec::new();
        }
        let budget = self.cfg.max_children;
        let mut out: Vec<(String, String)> = Vec::new();

        // 1. 网络词汇与网址协议特殊释义
        if word == "www." {
            return vec![("网址".to_string(), "World Wide Web 网址前缀".to_string())];
        }
        if word == "www" {
            return vec![
                ("中文".to_string(), "万维网".to_string()),
                ("全称".to_string(), "World Wide Web".to_string()),
            ];
        }
        if word == "http://" || word == "http" {
            return vec![("协议".to_string(), "超文本传输协议".to_string())];
        }
        if word == "https://" || word == "https" {
            return vec![("协议".to_string(), "安全超文本传输协议".to_string())];
        }

        // 2. 若候选词本身是英文单词（如 apple、computer、hello 等），其译文必须是中文释义
        let is_english = !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '\'');
        if is_english {
            for zh in self.glossary.english_to_chinese(word, budget) {
                if !out.iter().any(|(_, t)| t == &zh) {
                    out.push(("中文".to_string(), zh));
                }
                if out.len() >= budget {
                    break;
                }
            }
            return out;
        }

        // 3. 中文候选词：先查本地多语对照表（glossary.tsv / dict_en.tsv）
        for tr in self
            .glossary
            .translations(word, &self.cfg.targets, budget)
        {
            out.push((tr.lang, tr.text));
        }

        // 4. 若缺少英文译文，使用语气助词智能解析器（了、呢、的、得、吗、嘛等）及复合词切分兜底生成
        if !out.iter().any(|(l, _)| l.contains('英')) {
            let p_trans = particle::translate_particle_phrase(word, &self.glossary);
            for t in p_trans {
                if out.len() >= budget {
                    break;
                }
                if !out.iter().any(|(_, text)| text == &t) {
                    out.push(("英文".to_string(), t));
                }
            }
        }

        // 5. 远端条目只补空位、不挤本地条目
        if let Some(remote) = self.ai_cache.get(word) {
            for (lang, text) in remote {
                if out.len() >= budget {
                    break;
                }
                if out.iter().any(|(l, t)| l == lang || t == text) {
                    continue;
                }
                out.push((lang.clone(), text.clone()));
            }
        }
        out.truncate(budget);
        // 英文排第一条：连按一次数字就该拿到英文，而不是先撞上德语/日语。
        out.sort_by_key(|(lang, _)| if lang.contains('英') { 0 } else { 1 });
        out
    }

    /// Candidate words the AI should translate in the background.
    pub fn pending_ai_words(&self) -> Vec<String> {
        // AI 与免费网页翻译共用这条通道：任意一个开着就值得补译文。
        let remote_enabled = self.cfg.web_translate
            || (self.cfg.ai.enabled && !self.cfg.ai.api_key.trim().is_empty());
        if !remote_enabled || self.cfg.max_children == 0 {
            return Vec::new();
        }
        // 长句译文更重要：整句候选排在前面先翻。
        let mut out = Vec::new();
        let mut parents: Vec<&Row> = self.rows.iter().filter(|r| r.depth == 0).collect();
        parents.sort_by_key(|row| std::cmp::Reverse(row.text.chars().count()));
        // 一次把**这一页能看到的候选**都送出去翻，而不是只翻前两个：
        // 只翻两个时，用户点到的三四个词里总有没译文的，看起来就是「缺翻译」。
        let limit = self.cfg.page_size.max(self.cfg.ai.max_parents);
        for row in parents {
            if out.len() >= limit {
                break;
            }
            if self.ai_cache.contains_key(&row.text) || self.ai_requested.contains(&row.text) {
                continue;
            }
            let local = self
                .glossary
                .translations(&row.text, &self.cfg.targets, self.cfg.targets.len());
            if local.len() >= self.cfg.targets.len() {
                continue; // glossary already covers every target
            }
            out.push(row.text.clone());
        }
        out
    }

    pub fn mark_ai_requested(&mut self, word: &str) {
        if !self.ai_requested.iter().any(|w| w == word) {
            self.ai_requested.push(word.to_string());
        }
    }

    /// Merge AI translations and refresh the list when relevant.
    pub fn apply_ai_result(&mut self, word: &str, translations: Vec<(String, String)>) {
        if translations.is_empty() {
            return;
        }
        self.ai_cache.insert(word.to_string(), translations);
        if self.is_composing() {
            // Keep any digit selection the user already made.
            let selected = self.selected_group;
            let tier = self.selected_tier;
            let highlight = self.highlight;
            self.rebuild();
            self.selected_group = selected;
            self.selected_tier = tier;
            self.rebuild();
            self.highlight = highlight.min(self.rows.len().saturating_sub(1));
        }
    }

    /// Text for the candidate window's preedit line.
    pub fn preedit(&self) -> String {
        let label = self.pending();
        if label.is_empty() {
            self.composition.clone()
        } else {
            format!("{}  [{}]", self.composition, label)
        }
    }

    /// 学习这一次上屏：词频、上下文共现、生僻词编码。
    ///
    /// 只记中文（译文子候选、纯标点不进用户词库），并且只在确实有拼音composition
    /// 的时候记 —— 这样 `spellings` 里的 key 一定是用户真的敲过的拼音。
    fn record_learning(&mut self, text: &str) {
        if !self.cfg.learn_enabled || self.composition.is_empty() || text.is_empty() {
            return;
        }
        if !is_chinese_text(text) {
            return;
        }
        let prev = self.last_committed.clone();
        self.userdb
            .record_commit(&self.composition, text, prev.as_deref());
        self.last_committed = Some(text.to_string());
    }

    /// 生僻词兜底：当前拼音在词库里一个候选都没有（也没学过），要不要问 AI。
    pub fn pending_spelling_query(&self) -> Option<String> {
        if !self.cfg.ai.enabled || !self.cfg.ai_spell_fallback || !self.is_composing() {
            return None;
        }
        if self.composition.chars().count() < 3 || self.pool_len > 0 {
            return None;
        }
        if self.spelling_requested.iter().any(|c| c == &self.composition) {
            return None;
        }
        Some(self.composition.clone())
    }

    pub fn mark_spelling_requested(&mut self, composition: &str) {
        if !self.spelling_requested.iter().any(|c| c == composition) {
            self.spelling_requested.push(composition.to_string());
        }
    }

    /// AI 给出的生僻词候选：进候选列表，同时写进 `spellings` 变成离线知识。
    pub fn apply_spelling_result(&mut self, composition: &str, words: Vec<String>) {
        let words: Vec<String> = words
            .into_iter()
            .filter(|w| is_chinese_text(w))
            .take(self.cfg.max_parents)
            .collect();
        if words.is_empty() {
            return;
        }
        self.userdb.learn_spelling(composition, &words);
        if self.is_composing() {
            let selected = self.selected_group;
            let tier = self.selected_tier;
            let highlight = self.highlight;
            self.rebuild();
            self.selected_group = selected;
            self.selected_tier = tier;
            self.rebuild();
            self.highlight = highlight.min(self.rows.len().saturating_sub(1));
        }
    }

    /// 联想：上一个词的常见后继（给状态栏/日志/未来的预测条用）。
    pub fn predictions(&self, limit: usize) -> Vec<(String, u32)> {
        match self.last_committed.as_deref() {
            Some(prev) => self.userdb.predict_after(prev, limit),
            None => Vec::new(),
        }
    }

    /// `(词库条数, 用户词条数, 编码拼音数, 共现对数)`，probe 与日志用。
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        (
            self.dict.len(),
            self.userdb.word_count(),
            self.userdb.spelling_keys(),
            self.userdb.bigram_pairs(),
        )
    }
}

impl Engine {
    /// 标点映射：中文模式下把 ASCII 标点转成中文标点。
    ///
    /// 引号用开/关切换（第一次 “ 第二次 ”），和微软拼音一致；
    /// 其余是一对一映射，不进候选列表，直接上屏。
    /// 当前拼音串是否「像拼音」：词库里有没有它的匹配。
    ///
    /// `www` / `abc` 这类没有任何中文候选的串，标点要原样输出（网址、英文单词），
    /// 不能套用中文标点；`nihao` 有候选，就该按中文输入法转成 `。`。
    fn composition_looks_like_pinyin(&self) -> bool {
        !self.composition.is_empty() && !self.dict.lookup(&self.composition, 1, true).is_empty()
    }

    fn punctuation(&mut self, c: char) -> Option<String> {
        let mapped: &str = match c {
            ',' => "，",
            '.' => "。",
            ';' => "；",
            ':' => "：",
            '?' => "？",
            '!' => "！",
            '\\' => "、",
            '/' => "、",
            '(' => "（",
            ')' => "）",
            '[' => "【",
            ']' => "】",
            '{' => "「",
            '}' => "」",
            '<' => "《",
            '>' => "》",
            '-' => "－",
            '_' => "——",
            '=' => "＝",
            '+' => "＋",
            '|' => "｜",
            '`' => "·",
            '~' => "～",
            '^' => "……",
            '$' => "￥",
            '*' => "×",
            '"' => {
                let open = self.quote_double_open;
                self.quote_double_open = !open;
                return Some(if open { "“" } else { "”" }.to_string());
            }
            '\'' => {
                let open = self.quote_single_open;
                self.quote_single_open = !open;
                return Some(if open { "‘" } else { "’" }.to_string());
            }
            _ => return None,
        };
        Some(mapped.to_string())
    }
}

/// 全是中日韩统一表意文字（含扩展 A），用来区分「中文候选」和「译文候选」。
fn is_chinese_text(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| {
            matches!(c,
                '\u{3400}'..='\u{4DBF}'
                | '\u{4E00}'..='\u{9FFF}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{3007}')
        })
}

/// Chinese punctuation for characters typed while composing.
fn chinese_punctuation(c: char) -> Option<char> {
    Some(match c {
        ',' => '，',
        '.' => '。',
        '?' => '？',
        '!' => '！',
        ';' => '；',
        ':' => '：',
        _ => return None,
    })
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn engine() -> Engine {
        let dict = Dict::parse(
            "nihao\t你好\t10\n\
             nihao\t妳好\t500\n\
             ni\t你\t2\n",
        );
        let glossary = Glossary::parse(
            "你好\tde\thallo\n\
             你好\ten\thello\n\
             你好\tja\tこんにちは\n\
             妳好\tde\thallo\n",
        );
        Engine::new(dict, glossary, Config::default())
    }

    fn type_str(engine: &mut Engine, text: &str) {
        for c in text.chars() {
            engine.handle(Key::Char(c));
        }
    }

    #[test]
    fn typing_pinyin_builds_hierarchical_rows() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        let labels: Vec<&str> = e.rows().iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["1", "1.1", "1.2", "1.3", "2", "2.1"]);
        assert_eq!(e.rows()[0].text, "你好");
        // targets 默认英文优先，所以 1.1 是英文
        assert_eq!(e.rows()[1].text, "hello");
        assert_eq!(e.rows()[1].lang, "英文");
        assert_eq!(e.rows()[2].text, "hallo");
    }

    #[test]
    fn standalone_punctuation_converts_to_chinese() {
        let mut e = engine();
        assert_eq!(e.handle(Key::Char(',')), Outcome::Commit("，".to_string()));
        assert_eq!(e.handle(Key::Char('.')), Outcome::Commit("。".to_string()));
        assert_eq!(e.handle(Key::Char('/')), Outcome::Commit("、".to_string()));
        assert_eq!(e.handle(Key::Char('?')), Outcome::Commit("？".to_string()));
    }

    #[test]
    fn quotes_toggle_open_and_close() {
        let mut e = engine();
        assert_eq!(e.handle(Key::Char('"')), Outcome::Commit("“".to_string()));
        assert_eq!(e.handle(Key::Char('"')), Outcome::Commit("”".to_string()));
    }

    #[test]
    fn punctuation_appends_after_the_selected_candidate() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(e.handle(Key::Char(',')), Outcome::Commit("你好，".to_string()));
    }

    #[test]
    fn dash_pages_only_when_there_are_multiple_pages() {
        let mut raw = String::new();
        for i in 0..20 {
            raw.push_str(&format!("shi\t实{i:02}\t{}\n", 100 + i));
        }
        let mut e = Engine::new(Dict::parse(&raw), Glossary::default(), Config::default());
        type_str(&mut e, "shi");
        assert_eq!(e.handle(Key::Char('=')), Outcome::Consumed);
        assert_eq!(e.page(), 1, "多页时 `=` 用于翻页");
        assert_eq!(e.handle(Key::Char('-')), Outcome::Consumed);
        assert_eq!(e.page(), 0);

        // 只有一页时，`-` / `=` 仍作为翻页操作被吸收，绝不把未上屏候选和 ＝ 误上屏
        let mut single = engine();
        type_str(&mut single, "nihao");
        assert_eq!(single.handle(Key::Char('=')), Outcome::Consumed);
    }

    #[test]
    fn enter_commits_raw_pinyin_while_space_commits_candidate() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(e.handle(Key::Enter), Outcome::Commit("nihao".to_string()));

        let mut e2 = engine();
        type_str(&mut e2, "nihao");
        assert_eq!(e2.handle(Key::Space), Outcome::Commit("你好".to_string()));
    }

    #[test]
    fn empty_candidates_still_show_a_raw_row() {
        let mut e = engine();
        type_str(&mut e, "zzz");
        assert_eq!(e.rows().len(), 1, "无候选也要留一行原样拼音");
        assert_eq!(e.rows()[0].text, "zzz");
        assert_eq!(e.handle(Key::Space), Outcome::Commit("zzz".to_string()));
    }

    #[test]
    fn digit_selects_group_then_toggles_tier() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        // 第一次按 1：选中第 1 组，落在中文层
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1");
        // 再按一次 1：切到英文层
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1↓1", "第 1 条译文");
        // 再按一次：切到第 2 条译文
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1↓2");
        // 再按一次：切到第 3 条译文
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1↓3");
        // 再按一次：回到中文层
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1");
        // 候选列表保持 7 组可见，不再收缩
        let labels: Vec<&str> = e.rows().iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"2"), "不应收缩成只剩第一组");
    }

    #[test]
    fn digit_twice_then_space_commits_translation() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        e.handle(Key::Char('1'));
        e.handle(Key::Char('1')); // 切到英文层
        assert_eq!(e.handle(Key::Space), Outcome::Commit("hello".to_string()));
        assert!(!e.is_composing());
    }

    #[test]
    fn space_commits_top_candidate() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(e.handle(Key::Space), Outcome::Commit("你好".to_string()));
    }

    #[test]
    fn enter_commits_raw_even_with_selection() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        e.handle(Key::Char('1'));
        e.handle(Key::Char('1')); // 选到英文层
        // 回车按设计上屏原样拼音（选候选是空格的事）
        assert_eq!(e.handle(Key::Enter), Outcome::Commit("nihao".to_string()));
    }

    #[test]
    fn digit_without_translation_keeps_chinese_tier() {
        // 没有译文时，连按 1 也不会切到空的下层，空格仍然上屏中文
        let mut e = Engine::new(
            Dict::parse("wo\t我\t2\n"),
            Glossary::default(),
            Config::default(),
        );
        type_str(&mut e, "wo");
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1");
        assert_eq!(e.handle(Key::Char('1')), Outcome::Consumed);
        assert_eq!(e.pending(), "1", "没有译文就不该切层");
        assert_eq!(e.handle(Key::Space), Outcome::Commit("我".to_string()));
    }

    #[test]
    fn letter_after_digit_commits_then_starts_new_word() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        e.handle(Key::Char('1'));
        let outcome = e.handle(Key::Char('n'));
        assert_eq!(outcome, Outcome::Commit("你好".to_string()));
        assert_eq!(e.composition(), "n");
    }

    #[test]
    fn backspace_clears_selection_before_composition() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        e.handle(Key::Char('1'));
        e.handle(Key::Char('1')); // 第 1 条译文
        assert_eq!(e.pending(), "1↓1");
        e.handle(Key::Backspace);
        assert_eq!(e.pending(), "1", "先退回中文层");
        e.handle(Key::Backspace);
        assert_eq!(e.pending(), "", "再退取消选中");
        e.handle(Key::Backspace);
        assert_eq!(e.composition(), "niha", "最后才退拼音");
    }

    #[test]
    fn unknown_pinyin_is_passed_through_raw() {
        let mut e = engine();
        type_str(&mut e, "qqqq");
        // 无候选时保留一条「原样拼音」行，窗口不会消失
        assert_eq!(e.rows().len(), 1);
        assert_eq!(e.handle(Key::Space), Outcome::Commit("qqqq".to_string()));
    }

    #[test]
    fn ascii_mode_ignores_everything() {
        let mut e = engine();
        e.set_ascii_mode(true);
        assert_eq!(e.handle(Key::Char('n')), Outcome::Ignore);
        assert_eq!(e.handle(Key::Space), Outcome::Ignore);
    }

    #[test]
    fn chinese_punctuation_commits_and_appends() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(
            e.handle(Key::Char(',')),
            Outcome::Commit("你好，".to_string())
        );
    }

    #[test]
    fn digits_without_composition_pass_through() {
        let mut e = engine();
        assert_eq!(e.handle(Key::Char('5')), Outcome::Ignore);
    }

    #[test]
    fn clicking_a_child_commits_the_translation() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        // rows: 0=你好, 1=1.1 hello, 2=1.2 hallo, 3=1.3 こんにちは, 4=妳好 ...
        assert_eq!(e.commit_row(1).as_deref(), Some("hello"), "点下层=英文");
        assert!(!e.is_composing(), "点选后清空 composition");
    }

    #[test]
    fn clicking_a_parent_commits_the_chinese() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(e.commit_row(0).as_deref(), Some("你好"), "点上层=中文");
    }

    #[test]
    fn clicking_out_of_range_is_ignored() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        assert_eq!(e.commit_row(99), None);
        assert!(e.is_composing(), "无效点击不应清空状态");
    }

    #[test]
    fn pages_candidates_seven_per_page() {
        // 造 20 个同音候选，验证分页与页内重新编号
        let mut raw = String::new();
        for i in 0..20 {
            raw.push_str(&format!("shi\t实{i:02}\t{}\n", 100 + i));
        }
        let mut e = Engine::new(Dict::parse(&raw), Glossary::default(), Config::default());
        type_str(&mut e, "shi");

        assert_eq!(e.page_size(), 7);
        assert_eq!(e.page_count(), 3, "20 条候选 / 每页 7 = 3 页");
        let first: Vec<&str> = e
            .rows()
            .iter()
            .filter(|r| r.depth == 0)
            .map(|r| r.label.as_str())
            .collect();
        assert_eq!(first, vec!["1", "2", "3", "4", "5", "6", "7"]);
        assert_eq!(e.rows()[0].text, "实00");

        assert_eq!(e.handle(Key::PageNext), Outcome::Consumed);
        assert_eq!(e.page(), 1);
        let second: Vec<&str> = e
            .rows()
            .iter()
            .filter(|r| r.depth == 0)
            .map(|r| r.label.as_str())
            .collect();
        assert_eq!(second, vec!["1", "2", "3", "4", "5", "6", "7"], "页内重新编号");
        assert_eq!(e.rows()[0].text, "实07");

        assert_eq!(e.handle(Key::PagePrev), Outcome::Consumed);
        assert_eq!(e.page(), 0);
    }

    #[test]
    fn page_next_ignored_on_last_page() {
        let mut raw = String::new();
        for i in 0..8 {
            raw.push_str(&format!("shi\t实{i:02}\t{}\n", 100 + i));
        }
        let mut e = Engine::new(Dict::parse(&raw), Glossary::default(), Config::default());
        type_str(&mut e, "shi");
        assert_eq!(e.page_count(), 2);
        assert_eq!(e.handle(Key::PageNext), Outcome::Consumed);
        assert_eq!(e.page(), 1);
        assert_eq!(e.handle(Key::PageNext), Outcome::Ignore, "最后一页不再翻");
    }

    #[test]
    fn default_highlight_is_never_a_translation() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        // 刚打出来：高亮必须在中文那一行
        assert_eq!(e.rows()[e.highlight()].depth, 0);
        assert_eq!(e.rows()[e.highlight()].text, "你好");

        // 远端译文回来重排后，也不能把高亮挪到译文上
        e.apply_ai_result("你好", vec![("德语".into(), "hallo".into())]);
        assert_eq!(
            e.rows()[e.highlight()].depth,
            0,
            "默认选中状态只能是中文，不能是译文"
        );
    }

    /// 网址/英文串里的 `.` 必须原样输出，不能被当成中文句号。
    #[test]
    fn dot_stays_ascii_for_non_pinyin_input() {
        let mut e = engine();
        type_str(&mut e, "www");
        assert_eq!(
            e.handle(Key::Char('.')),
            Outcome::Commit("www.".to_string()),
            "www 后面不能变成 www。"
        );

        // 正常拼音仍然转中文标点
        type_str(&mut e, "nihao");
        assert_eq!(e.handle(Key::Char('.')), Outcome::Commit("你好。".to_string()));
    }

    /// 打英文时不该在中文候选下面再挂一行英文（那正是用户刚打的词）。
    #[test]
    fn english_input_does_not_echo_english_as_translation() {
        let dict = Dict::parse("nihao\t你好\t10\n");
        let glossary = Glossary::parse("你好\ten\thello\n你好\tde\thallo\n");
        let mut e = Engine::new(dict, glossary, Config::default());
        // 反查：hello -> 你好；这条候选下面不该再出现「英文 hello」
        type_str(&mut e, "hello");
        let echoed = e
            .rows()
            .iter()
            .filter(|r| r.depth == 1)
            .any(|r| r.text.eq_ignore_ascii_case("hello"));
        assert!(!echoed, "打英文时不应把英文本身当译文挂上去");
    }

    #[test]
    fn ai_results_extend_children() {
        let mut e = engine();
        type_str(&mut e, "nihao");
        // German/English/Japanese already come from the glossary, so an AI entry
        // duplicating them must not appear twice under candidate 1.
        e.apply_ai_result("你好", vec![("德语".into(), "hallo".into())]);
        let children: Vec<&str> = e
            .rows()
            .iter()
            .filter(|r| r.depth == 1 && r.label.starts_with("1."))
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(children, vec!["hello", "hallo", "こんにちは"]);
    }

    #[test]
    fn ai_translation_fills_missing_target() {
        let mut e = Engine::new(
            Dict::parse("xiexie\t谢谢\t20\n"),
            Glossary::default(),
            Config::default(),
        );
        type_str(&mut e, "xiexie");
        assert!(e.rows().iter().all(|r| r.depth == 0));
        e.apply_ai_result("谢谢", vec![("德语".into(), "danke".into())]);
        let children: Vec<&str> = e
            .rows()
            .iter()
            .filter(|r| r.depth == 1)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(children, vec!["danke"]);
    }
}
