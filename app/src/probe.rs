//! `--probe`：不开窗口，直接把候选树打出来 —— 自检、CI 与截图验证都靠它。
//!
//! 这也是「自学习」的可观测面：`--learn` 会真的把词写进用户词库，
//! 再 `--probe` 就能看到排序变化与新学到的生僻词。

use std::path::PathBuf;

use mcte_core::config::{self, Config};
use mcte_core::dict::Dict;
use mcte_core::engine::{Engine, Key, Outcome};
use mcte_core::glossary::Glossary;
use mcte_core::userdb::UserDb;

use crate::hook;

#[derive(Default, Clone)]
pub struct Options {
    /// `--commit <label>`：模拟按下某个层级标签后上屏。
    pub commit: Option<String>,
    /// `--userdb <path>`：用临时用户词库，避免污染真实数据。
    pub userdb: Option<PathBuf>,
    /// `--no-prefix`：关掉前缀匹配。
    pub no_prefix: bool,
    /// `--page N`：列出第 N 页（1 起）的候选。
    pub page: usize,
    /// `--ascii`：在英语模式下打字
    pub ascii: bool,
}

impl Options {
    pub fn from_args(args: &[String]) -> Self {
        let mut options = Options {
            commit: None,
            userdb: None,
            no_prefix: false,
            page: 1,
            ascii: false,
        };
        let mut idx = 0;
        while idx < args.len() {
            match args[idx].as_str() {
                "--commit" => {
                    options.commit = args.get(idx + 1).cloned();
                    idx += 2;
                }
                "--userdb" => {
                    options.userdb = args.get(idx + 1).map(PathBuf::from);
                    idx += 2;
                }
                "--no-prefix" => {
                    options.no_prefix = true;
                    idx += 1;
                }
                "--page" => {
                    options.page = args.get(idx + 1).and_then(|n| n.parse().ok()).unwrap_or(1);
                    idx += 2;
                }
                "--ascii" => {
                    options.ascii = true;
                    idx += 1;
                }
                _ => idx += 1,
            }
        }
        options
    }
}

fn userdb_path(override_path: Option<PathBuf>) -> PathBuf {
    override_path.unwrap_or_else(|| config::app_data_dir().join("userdb.json"))
}

fn build_engine(options: &Options) -> Result<Engine, String> {
    let data_dir = config::data_dir();
    let dict_path = data_dir.join("dict.txt");
    let dict = Dict::load(&dict_path)
        .map_err(|e| format!("cannot load dictionary {}: {e}", dict_path.display()))?;
    let glossary = Glossary::from_dir(&data_dir);
    let mut cfg = Config::load_or_create(&config::app_data_dir());
    if options.no_prefix {
        cfg.prefix_search = false;
    }
    let userdb = UserDb::open(&userdb_path(options.userdb.clone()));
    Ok(Engine::with_userdb(dict, glossary, cfg, userdb))
}

fn type_pinyin(engine: &mut Engine, input: &str) {
    for c in input.chars() {
        engine.handle(Key::Char(c));
    }
}

pub fn run(input: &str, options: Options) -> Result<(), String> {
    let mut engine = build_engine(&options)?;
    if options.ascii {
        engine.set_ascii_mode(true);
    }
    type_pinyin(&mut engine, input);
    // 翻到指定页（`-` / `+` 就是 PagePrev / PageNext）
    while engine.page() + 1 < options.page {
        engine.handle(Key::PageNext);
    }

    println!("input: {input}");
    let (dict_rows, user_words, spellings, bigrams) = engine.stats();
    println!(
        "dict : {dict_rows} rows | user: {user_words} words / {spellings} spellings / {bigrams} bigrams"
    );
    if let Some(prev) = engine.last_committed() {
        println!("prev : {prev}");
    }
    println!(
        "rows : {}   page: {}/{}",
        engine.rows().len(),
        engine.page() + 1,
        engine.page_count()
    );
    for (idx, row) in engine.rows().iter().enumerate() {
        let marker = if idx == engine.highlight() { ">" } else { " " };
        if row.depth == 0 {
            println!("{marker} {:<4}{}", row.label, row.text);
        } else {
            // 新模型里译文与所属组共用编号：显示 `↓1 hello`
            let group = row.label.split('.').next().unwrap_or("");
            println!("{marker} ↓{:<3}{}   ({})", group, row.text, row.lang);
        }
    }

    // 逐个模拟按键，报告最终上屏文本 —— 测试可以断言这里的字符串。
    for label in ["1", "1.1", "1.2", "2"] {
        let mut clone = engine.clone();
        let mut outcome = Outcome::Consumed;
        for c in label.chars() {
            outcome = clone.handle(Key::Char(c));
            if matches!(outcome, Outcome::Commit(_)) {
                break;
            }
        }
        if !matches!(outcome, Outcome::Commit(_)) {
            outcome = clone.handle(Key::Space);
        }
        println!("key {label:<4}-> {}", describe(&outcome));
    }

    if let Some(label) = options.commit {
        let mut outcome = Outcome::Consumed;
        for c in label.chars() {
            outcome = engine.handle(Key::Char(c));
            if matches!(outcome, Outcome::Commit(_)) {
                break;
            }
        }
        if !matches!(outcome, Outcome::Commit(_)) {
            outcome = engine.handle(Key::Space);
        }
        println!("commit {label:<4}-> {}", describe(&outcome));
        let _ = engine.flush_userdb();
        let (_, user_words, spellings, bigrams) = engine.stats();
        println!("after: user: {user_words} words / {spellings} spellings / {bigrams} bigrams");
        println!("predict after: {:?}", engine.predictions(3));
    }

    println!("hook : {}", hook::describe_backend());
    Ok(())
}

/// `--learn <拼音> <词> [n]`：把这组「拼音 → 词」写进用户词库，再打印候选与联想。
pub fn learn(pinyin: &str, word: &str, times: u32) -> Result<(), String> {
    if pinyin.is_empty() || word.is_empty() {
        return Err("用法: mcte --learn <拼音> <词> [次数]".to_string());
    }
    let mut engine = build_engine(&Options {
        commit: None,
        userdb: None,
        no_prefix: false,
        page: 1,
        ..Default::default()
    })?;
    for _ in 0..times.max(1) {
        let prev = engine.last_committed().map(str::to_string);
        engine
            .userdb_mut()
            .record_commit(pinyin, word, prev.as_deref());
    }
    engine.flush_userdb().map_err(|e| e.to_string())?;

    let (dict_rows, user_words, spellings, bigrams) = engine.stats();
    println!("learned: {pinyin} -> {word} x{times}");
    println!(
        "dict : {dict_rows} rows | user: {user_words} words / {spellings} spellings / {bigrams} bigrams"
    );

    let mut engine = build_engine(&Options {
        commit: None,
        userdb: None,
        no_prefix: false,
        page: 1,
        ..Default::default()
    })?;
    type_pinyin(&mut engine, pinyin);
    println!("rows after learning:");
    for row in engine.rows() {
        if row.depth == 0 {
            println!("  {:<4}{}", row.label, row.text);
        }
    }
    Ok(())
}

/// `--eval <cases.tsv>`：跑评测集并输出 top1/top3 准确率。
///
/// 每行：`mode<TAB>输入<TAB>期望`，mode 取值：
/// * `segment` 长串拆解：期望 top-1 等于 `期望`
/// * `en2zh`   英文反查：期望 `期望` 出现在 top-3（同时统计 top-1）
/// * `zh2en`   中文→英文：输入拼音，期望「中文|英文」——候选里有这个中文，
///             且它的英文子候选包含该英文
pub fn eval(path: &std::path::Path) -> Result<(), String> {
    let mut engine = build_engine(&Options {
        commit: None,
        userdb: Some(std::env::temp_dir().join("mcte-eval-userdb.json")),
        no_prefix: false,
        page: 1,
        ..Default::default()
    })?;
    let _ = std::fs::remove_file(std::env::temp_dir().join("mcte-eval-userdb.json"));

    let raw = std::fs::read_to_string(path).map_err(|e| format!("读取评测集失败: {e}"))?;
    let mut total = 0usize;
    let mut top1 = 0usize;
    let mut top3 = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let (mode, input, expected) = (parts[0].trim(), parts[1].trim(), parts[2].trim());

        engine.clear_userdb();
        engine.abort();
        for c in input.chars() {
            engine.handle(Key::Char(c));
        }
        let parents: Vec<String> = engine
            .rows()
            .iter()
            .filter(|row| row.depth == 0)
            .map(|row| row.text.clone())
            .collect();
        let children: Vec<String> = engine
            .rows()
            .iter()
            .filter(|row| row.depth == 1)
            .map(|row| row.text.clone())
            .collect();

        total += 1;
        // `~` 分隔多个可接受答案；zh2en 的格式是 `中文|英文1~英文2`
        let alternatives: Vec<&str> = expected.split('~').map(str::trim).collect();
        let hit1 = match mode {
            "segment" | "en2zh" => parents
                .first()
                .map(|p| alternatives.iter().any(|alt| alt == p))
                .unwrap_or(false),
            "zh2en" => {
                let (zh, en) = expected.split_once('|').unwrap_or((expected, ""));
                parents.iter().take(1).any(|p| p == zh)
                    && en
                        .split('~')
                        .any(|alt| children.iter().any(|c| c.to_lowercase().contains(&alt.to_lowercase())))
            }
            _ => false,
        };
        let hit3 = match mode {
            "segment" | "en2zh" => parents
                .iter()
                .take(3)
                .any(|p| alternatives.iter().any(|alt| alt == p)),
            "zh2en" => {
                let (zh, en) = expected.split_once('|').unwrap_or((expected, ""));
                parents.iter().take(3).any(|p| p == zh)
                    && en
                        .split('~')
                        .any(|alt| children.iter().any(|c| c.to_lowercase().contains(&alt.to_lowercase())))
            }
            _ => false,
        };
        if hit1 {
            top1 += 1;
        }
        if hit3 {
            top3 += 1;
        } else {
            failures.push(format!(
                "  line {:<4} {:<8} {:<22} 期望 {:<14} 实际 {}",
                line_no + 1,
                mode,
                input,
                expected,
                parents.iter().take(3).cloned().collect::<Vec<_>>().join(" / ")
            ));
        }
    }

    println!("评测集: {}", path.display());
    println!("用例数: {total}");
    if total > 0 {
        println!(
            "top-1 命中: {top1}/{total} = {:.1}%",
            top1 as f64 * 100.0 / total as f64
        );
        println!(
            "top-3 命中: {top3}/{total} = {:.1}%",
            top3 as f64 * 100.0 / total as f64
        );
    }
    if !failures.is_empty() {
        println!("\n未命中（最多列 40 条）:");
        for line in failures.iter().take(40) {
            println!("{line}");
        }
    }
    Ok(())
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Commit(text) => format!("commit {text:?}"),
        Outcome::Consumed => "(consumed, no commit)".to_string(),
        Outcome::Ignore => "(ignored)".to_string(),
    }
}


/// 在 `engine` 的副本上敲一遍 `pinyin`，返回一级候选文本。
fn typed_rows(engine: &Engine, pinyin: &str) -> Vec<String> {
    let mut clone = engine.clone();
    for c in pinyin.chars() {
        clone.handle(Key::Char(c));
    }
    clone
        .rows()
        .iter()
        .filter(|row| row.depth == 0)
        .map(|row| row.text.clone())
        .collect()
}

/// 敲 `pinyin` 并用 `label` 选词上屏（必要时补一个空格），返回上屏文本。
fn typed_commit(engine: &mut Engine, pinyin: &str, label: &str) -> Option<String> {
    for c in pinyin.chars() {
        engine.handle(Key::Char(c));
    }
    for c in label.chars() {
        if let Outcome::Commit(text) = engine.handle(Key::Char(c)) {
            return Some(text);
        }
    }
    if let Outcome::Commit(text) = engine.handle(Key::Space) {
        return Some(text);
    }
    None
}

/// `--selftest`：一个进程里跑完「词频 → 上下文联想 → 生僻词」三段剧本并断言。
/// 退出码即结论，CI 与发布前自检都用它。
pub fn selftest() -> Result<(), String> {
    let mut failures = 0usize;
    let fresh = || -> Result<Engine, String> {
        let mut engine = build_engine(&Options {
            commit: None,
            userdb: None,
            no_prefix: false,
            page: 1,
            ..Default::default()
        })?;
        engine.clear_userdb();
        Ok(engine)
    };

    // ---- 1. 词频排序：反复上屏同一个词，它应该往前挪 ----
    let mut engine = fresh()?;
    let before = typed_rows(&engine, "xiexie");
    for _ in 0..15 {
        let prev = engine.last_committed().map(str::to_string);
        engine
            .userdb_mut()
            .record_commit("xiexie", "写写", prev.as_deref());
    }
    let after = typed_rows(&engine, "xiexie");
    let before_pos = before.iter().position(|w| w == "写写");
    let after_pos = after.iter().position(|w| w == "写写");
    let ok = matches!((before_pos, after_pos), (Some(b), Some(a)) if a < b);
    report(
        "词频排序把常用词提前",
        ok,
        format!("写写 before=#{:?} after=#{:?}", before_pos, after_pos),
        &mut failures,
    );

    // ---- 2. 上下文联想：打完「你好」后「世界」应该更靠前 ----
    let plain_engine = fresh()?;
    let plain = typed_rows(&plain_engine, "shijie");
    let mut ctx_engine = fresh()?;
    typed_commit(&mut ctx_engine, "nihao", "1"); // 上一个词：你好
    for _ in 0..6 {
        let prev = ctx_engine.last_committed().map(str::to_string);
        ctx_engine
            .userdb_mut()
            .record_commit("shijie", "师姐", prev.as_deref());
    }
    let contextual = typed_rows(&ctx_engine, "shijie");
    let plain_pos = plain.iter().position(|w| w == "师姐");
    let ctx_pos = contextual.iter().position(|w| w == "师姐");
    let ok = matches!((plain_pos, ctx_pos), (Some(p), Some(c)) if c < p);
    report(
        "上下文联想把后继词提前",
        ok,
        format!("师姐 plain=#{plain_pos:?} contextual=#{ctx_pos:?}"),
        &mut failures,
    );

    // ---- 3. 生僻词自动编码：词库没有的拼音，学一次就进候选 ----
    let mut engine = fresh()?;
    // 用「正常分词拼不出来」的键，专门验证 spellings（自动编码）这条链路。
    // （`zhuanyebankuai` 现在会被分词组句直接拼出「专业板块」，不适合当这个用例。）
    let pinyin = "zhuanyebankuaii";
    let unknown = typed_rows(&engine, pinyin);
    let before = unknown.iter().any(|w| w == "专业板块");
    engine.apply_spelling_result(pinyin, vec!["专业板块".to_string()]);
    let learned = typed_rows(&engine, pinyin);
    let after = learned.iter().any(|w| w == "专业板块");
    report(
        "生僻词学习后进入候选",
        !before && after,
        format!("before={before} after={after} top={:?}", learned.first()),
        &mut failures,
    );

    // ---- 4. 中文与译文分别落库：译文不该污染用户词库 ----
    let mut engine = fresh()?;
    engine.userdb_mut().record_commit("nihao", "你好", None);
    let words_before = engine.stats().1;
    let _ = typed_commit(&mut engine, "nihao", "1.1"); // 上屏 hallo
    let words_after = engine.stats().1;
    report(
        "译文上屏不写入用户词库",
        words_after == words_before,
        format!("words {words_before} -> {words_after}"),
        &mut failures,
    );

    println!();
    if failures > 0 {
        println!("SELFTEST FAILED ({failures})");
        Err(format!("{failures} checks failed"))
    } else {
        println!("SELFTEST PASSED");
        Ok(())
    }
}

fn report(name: &str, ok: bool, detail: String, failures: &mut usize) {
    if ok {
        println!("PASS  {name}  {detail}");
    } else {
        *failures += 1;
        println!("FAIL  {name}  {detail}");
    }
}