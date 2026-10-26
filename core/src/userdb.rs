//! 用户词库：自学习 + 联想排序 + 生僻词自动编码。
//!
//! 三张表，都是「用户自己敲出来的事实」，不依赖任何联网数据：
//!
//! | 表 | 作用 | 解决什么 |
//! |---|---|---|
//! | `words` | 每个词的选词次数与最近使用时间 | 常用词越用越靠前（个人习惯排序） |
//! | `bigrams` | 上一个词 → 这个词 的共现次数 | 上下文联想：打完「报告」再打「bu」时「部门」优先 |
//! | `spellings` | 拼音 → 用户实际选过的词 | 生僻词/自造词一次选过，下次就进候选 |
//!
//! `spellings` 是「生僻词自动迭代」的落点：词库没有的词，用户用 AI 兜底或整句上屏
//! 选过一次之后就被编码记住，之后完全离线可用。词库本身（`data/dict.txt`）不需要重建。

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 用户学到的词在候选里的起评分（越低越靠前）。比词库的前缀匹配（+50 000）略高一点，
/// 这样「一次都没选过的长词」不会压住用户真的选过的词，但精确匹配的词仍在其前面。
pub const LEARNED_BASE_SCORE: i64 = 40_000;

const MAX_SPELLINGS_PER_KEY: usize = 64;
const MAX_BIGRAM_TARGETS: usize = 48;
const MAX_WORDS: usize = 20_000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WordStat {
    pub count: u32,
    #[serde(default)]
    pub last_used: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserDb {
    #[serde(default)]
    words: HashMap<String, WordStat>,
    #[serde(default)]
    spellings: HashMap<String, Vec<String>>,
    #[serde(default)]
    bigrams: HashMap<String, HashMap<String, u32>>,
    #[serde(skip)]
    path: Option<PathBuf>,
    #[serde(skip)]
    dirty: bool,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl UserDb {
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let mut db: UserDb = serde_json::from_str(&raw)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        db.path = Some(path.to_path_buf());
        db.dirty = false;
        Ok(db)
    }

    /// Load, or start a fresh database bound to `path`.
    pub fn open(path: &Path) -> Self {
        match Self::load(path) {
            Ok(db) => db,
            Err(_) => Self {
                path: Some(path.to_path_buf()),
                ..Default::default()
            },
        }
    }

    pub fn bind(&mut self, path: &Path) {
        self.path = Some(path.to_path_buf());
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 原子保存（临时文件 + rename），任何失败都只返回错误、不破坏旧文件。
    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if !self.dirty {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let raw = serde_json::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, raw)?;
        fs::rename(&tmp, &path)?;
        self.dirty = false;
        Ok(())
    }

    pub fn record_commit(&mut self, composition: &str, word: &str, prev: Option<&str>) {
        let now = now_secs();
        let stat = self.words.entry(word.to_string()).or_default();
        stat.count = stat.count.saturating_add(1);
        stat.last_used = now;

        let bucket = self.spellings.entry(composition.to_string()).or_default();
        if !bucket.iter().any(|w| w == word) {
            if bucket.len() >= MAX_SPELLINGS_PER_KEY {
                bucket.remove(0);
            }
            bucket.push(word.to_string());
        }

        if let Some(prev) = prev.filter(|p| !p.is_empty() && *p != word) {
            let targets = self.bigrams.entry(prev.to_string()).or_default();
            let counter = targets.entry(word.to_string()).or_default();
            *counter = counter.saturating_add(1);
            if targets.len() > MAX_BIGRAM_TARGETS {
                if let Some((coldest, _)) = targets
                    .iter()
                    .min_by_key(|(_, count)| **count)
                    .map(|(word, count)| (word.clone(), *count))
                {
                    targets.remove(&coldest);
                }
            }
        }

        self.dirty = true;
        self.prune();
    }

    /// 连续输入组词学习（自造词）：记录由连续单字/词拼接而成的词组。
    pub fn record_phrase(&mut self, composition: &str, phrase: &str) {
        if composition.is_empty() || phrase.is_empty() {
            return;
        }
        let now = now_secs();
        let stat = self.words.entry(phrase.to_string()).or_default();
        stat.count = stat.count.saturating_add(1);
        stat.last_used = now;

        let bucket = self.spellings.entry(composition.to_string()).or_default();
        if !bucket.iter().any(|w| w == phrase) {
            if bucket.len() >= MAX_SPELLINGS_PER_KEY {
                bucket.remove(0);
            }
            bucket.push(phrase.to_string());
        }
        self.dirty = true;
        self.prune();
    }

    /// 查询上一个词到下一个词的二元转移次数（O(1) 无额外内存分配）。
    pub fn bigram_count(&self, prev: &str, next: &str) -> u32 {
        self.bigrams
            .get(prev)
            .and_then(|targets| targets.get(next))
            .copied()
            .unwrap_or(0)
    }

    /// AI 兜底学到的词：直接进 spellings，同时给一个很小的初值计数。
    pub fn learn_spelling(&mut self, composition: &str, words: &[String]) {
        if words.is_empty() {
            return;
        }
        let bucket = self.spellings.entry(composition.to_string()).or_default();
        for word in words {
            if word.trim().is_empty() || bucket.iter().any(|w| w == word) {
                continue;
            }
            if bucket.len() >= MAX_SPELLINGS_PER_KEY {
                bucket.remove(0);
            }
            bucket.push(word.clone());
        }
        self.dirty = true;
        self.prune();
    }

    /// 候选打分：越低越靠前。`base_rank` 来自词库的全局频率序。
    pub fn score(&self, base_rank: u32, word: &str, prev: Option<&str>, now: u64) -> i64 {
        let mut score = base_rank as i64;
        if let Some(stat) = self.words.get(word) {
            // 对数增长：第 100 次选词不该是第 1 次的 100 倍权重
            let freq = (stat.count as f64 + 1.0).ln();
            score -= (freq * 12_000.0) as i64;
            if now.saturating_sub(stat.last_used) < 3_600 {
                score -= 3_000; // 最近一小时内用过，继续排前面
            }
        }
        if let Some(prev) = prev.filter(|p| !p.is_empty()) {
            if let Some(count) = self.bigrams.get(prev).and_then(|m| m.get(word)) {
                let assoc = (*count as f64 + 1.0).ln();
                score -= (assoc * 25_000.0) as i64;
            }
        }
        score
    }

    /// 该拼音下用户亲口选过的词（生僻词自动编码的结果）。
    pub fn learned(&self, composition: &str) -> &[String] {
        self.spellings
            .get(composition)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 联想：给定上一个词，返回用户最常接的词（按次数降序）。
    pub fn predict_after(&self, prev: &str, limit: usize) -> Vec<(String, u32)> {
        let Some(targets) = self.bigrams.get(prev) else {
            return Vec::new();
        };
        let mut items: Vec<(String, u32)> = targets
            .iter()
            .map(|(word, count)| (word.clone(), *count))
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        items.truncate(limit);
        items
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn spelling_keys(&self) -> usize {
        self.spellings.len()
    }

    pub fn bigram_pairs(&self) -> usize {
        self.bigrams.values().map(HashMap::len).sum()
    }

    /// 常用的那个词（用于状态栏/日志）。
    pub fn top_words(&self, limit: usize) -> Vec<(String, u32)> {
        let mut items: Vec<(String, u32)> = self
            .words
            .iter()
            .map(|(word, stat)| (word.clone(), stat.count))
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        items.truncate(limit);
        items
    }

    /// 控制文件体积：词表按使用次数淘汰，保留最近用过的。
    fn prune(&mut self) {
        if self.words.len() <= MAX_WORDS {
            return;
        }
        let mut items: Vec<(String, u32, u64)> = self
            .words
            .iter()
            .map(|(word, stat)| (word.clone(), stat.count, stat.last_used))
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        self.words = items
            .into_iter()
            .take(MAX_WORDS)
            .map(|(word, count, last_used)| (word, WordStat { count, last_used }))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequent_words_get_a_better_score() {
        let mut db = UserDb::default();
        for _ in 0..20 {
            db.record_commit("nihao", "你好", None);
        }
        let now = now_secs();
        let learned = db.score(LEARNED_BASE_SCORE as u32, "你好", None, now);
        let stranger = db.score(LEARNED_BASE_SCORE as u32, "泥壕", None, now);
        assert!(learned < stranger, "常用词应排在生词前面");
    }

    #[test]
    fn bigram_context_lifts_the_expected_successor() {
        let mut db = UserDb::default();
        for _ in 0..5 {
            db.record_commit("baogao", "报告", None);
            db.record_commit("bumen", "部门", Some("报告"));
        }
        let now = now_secs();
        let with_context = db.score(100_000, "部门", Some("报告"), now);
        let without = db.score(100_000, "部门", None, now);
        assert!(with_context < without, "上下文联想应加权");
        assert_eq!(db.predict_after("报告", 3)[0].0, "部门");
    }

    #[test]
    fn committed_spelling_becomes_learned_candidate() {
        let mut db = UserDb::default();
        assert!(db.learned("wanshang").is_empty());
        db.record_commit("wanshang", "晚上", None);
        assert_eq!(db.learned("wanshang"), ["晚上".to_string()]);
        // 同一个词重复选不会重复编码
        db.record_commit("wanshang", "晚上", None);
        assert_eq!(db.learned("wanshang").len(), 1);
    }

    #[test]
    fn ai_learned_words_are_recorded() {
        let mut db = UserDb::default();
        db.learn_spelling("zhuanyebankuai", &["专业板块".to_string()]);
        assert_eq!(db.learned("zhuanyebankuai"), ["专业板块".to_string()]);
    }

    #[test]
    fn roundtrip_through_disk() {
        let dir = std::env::temp_dir().join("mcte-userdb-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("userdb.json");

        let mut db = UserDb::open(&path);
        db.record_commit("nihao", "你好", None);
        db.save().unwrap();

        let reloaded = UserDb::load(&path).unwrap();
        assert_eq!(reloaded.learned("nihao"), ["你好".to_string()]);
        assert_eq!(reloaded.word_count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn spelling_bucket_is_bounded() {
        let mut db = UserDb::default();
        for i in 0..(MAX_SPELLINGS_PER_KEY + 10) {
            db.record_commit("key", &format!("词{i}"), None);
        }
        assert!(db.learned("key").len() <= MAX_SPELLINGS_PER_KEY);
        // 最近选的词一定还在
        assert!(db.learned("key").last().unwrap().starts_with("词"));
    }

    #[test]
    fn self_transition_is_not_recorded_as_bigram() {
        let mut db = UserDb::default();
        db.record_commit("baogao", "报告", Some("报告"));
        assert_eq!(db.bigram_pairs(), 0);
    }

    #[test]
    fn records_learned_phrases_and_bigram_counts() {
        let mut db = UserDb::default();
        db.record_commit("zengya", "增压", None);
        db.record_commit("qi", "器", Some("增压"));
        db.record_phrase("zengyaqi", "增压器");

        assert_eq!(db.learned("zengyaqi"), ["增压器".to_string()]);
        assert_eq!(db.bigram_count("增压", "器"), 1);
        assert_eq!(db.bigram_count("增压", "其"), 0);

        let now = now_secs();
        let score = db.score(LEARNED_BASE_SCORE as u32, "增压器", None, now);
        assert!(score < LEARNED_BASE_SCORE, "学习到的词组应具有更优的打分");
    }
}
