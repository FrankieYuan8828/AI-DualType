//! 拼音分词（长串拆解 / 组句）。
//!
//! 用户打 `bukeyitihuan` 时，词库里没有这条整串，以前一个候选都不出。
//! 这里做 Rime 那套「组句」的最小实现：把连续拼音切成若干词，用词频做代价
//! （Viterbi / k-best），输出多条切分：
//!
//! ```text
//! bukeyitihuan
//!   不可以 + 替换     ← 代价最低
//!   不可以 + 提换
//!   不 + 可以 + 替换
//!   不可 + 以 + 替换
//! ```
//!
//! 不需要单独的音节表：**词库的 key 本身就是音节拼接**，
//! 所以「`s[p..q]` 是词库里的 key」等价于「这一段是合法音节组合」。

use std::collections::HashMap;

use crate::dict::Dict;

/// 每个词额外加一点代价，避免系统偏袒「全部切成单字」。
const WORD_PENALTY: f64 = 8.5;
/// 一条候选最多拆成几个词。
const MAX_WORDS: usize = 6;
/// 单个词最多多少字符（拼音串长度上限）。
const MAX_WORD_CHARS: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Segmentation {
    /// 拼起来的中文（如「不可以替换」）
    pub text: String,
    /// 拆出来的词（如 ["不可以", "替换"]）
    pub words: Vec<String>,
    /// 代价：越低越可信
    pub cost: f64,
}

/// 把 `pinyin` 拆成最多 `k` 条候选；没有可行切分时返回空。
pub fn segment(dict: &Dict, pinyin: &str, k: usize) -> Vec<Segmentation> {
    segment_with_userdb(dict, None, pinyin, k)
}

/// 支持结合用户词库（已学词 + 二元转移共现加权）的拼音分词
pub fn segment_with_userdb(
    dict: &Dict,
    userdb: Option<&crate::userdb::UserDb>,
    pinyin: &str,
    k: usize,
) -> Vec<Segmentation> {
    let chars: Vec<char> = pinyin.chars().collect();
    let n = chars.len();
    if n < 2 || k == 0 {
        return Vec::new();
    }

    // key(拼音) -> Vec<(中文词, rank)>：切分时探索高频候选（如 chuang -> 窗 / 床 / 创）
    let mut cache: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    let mut lookup = |key: &str| -> Vec<(String, u32)> {
        if let Some(hit) = cache.get(key) {
            return hit.clone();
        }
        let mut found: Vec<(String, u32)> = dict
            .top_entries(key, 16)
            .into_iter()
            .map(|(word, rank)| (word.to_string(), rank))
            .collect();
        if let Some(userdb) = userdb {
            for learned_word in userdb.learned(key) {
                if !found.iter().any(|(w, _)| w == learned_word) {
                    found.insert(0, (learned_word.clone(), 1000));
                }
            }
        }
        cache.insert(key.to_string(), found.clone());
        found
    };

    // paths[i]：到达位置 i 的若干条最优路径
    let mut paths: Vec<Vec<(f64, Vec<String>)>> = vec![Vec::new(); n + 1];
    paths[0].push((0.0, Vec::new()));

    for start in 0..n {
        if paths[start].is_empty() {
            continue;
        }
        let tails = paths[start].clone();
        let max_end = (start + MAX_WORD_CHARS).min(n);
        for end in (start + 1)..=max_end {
            let key: String = chars[start..end].iter().collect();
            let entries = lookup(&key);
            if entries.is_empty() {
                continue;
            }
            for (word, rank) in entries {
                let base_cost = (rank as f64 + 1.0).ln() + WORD_PENALTY;
                for (base, words) in &tails {
                    if words.len() >= MAX_WORDS {
                        continue;
                    }
                    let mut transition_bonus = 0.0;
                    if let Some(userdb) = userdb {
                        if let Some(prev_word) = words.last() {
                            let count = userdb.bigram_count(prev_word, &word);
                            if count > 0 {
                                transition_bonus = (count as f64 + 1.0).ln() * 3.5;
                            }
                        }
                    }
                    let step_cost = (base_cost - transition_bonus).max(0.1);
                    let mut next = words.clone();
                    next.push(word.clone());
                    push_best(&mut paths[end], base + step_cost, next, k);
                }
            }
        }
    }

    let mut out: Vec<Segmentation> = paths[n]
        .drain(..)
        .map(|(cost, words)| Segmentation {
            text: words.concat(),
            words,
            cost,
        })
        .collect();
    out.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| a.text == b.text);
    out.truncate(k);
    out
}

/// 同类代价下保留最好的 `k` 条；相同切分（词序列相同）只留最优。
fn push_best(slot: &mut Vec<(f64, Vec<String>)>, cost: f64, words: Vec<String>, k: usize) {
    if let Some(index) = slot.iter().position(|(_, w)| *w == words) {
        if cost < slot[index].0 {
            slot[index] = (cost, words);
        }
    } else {
        slot.push((cost, words));
    }
    slot.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    slot.truncate(k);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict() -> Dict {
        Dict::parse(
            "bu\t不\t5\n\
             ke\t可\t30\n\
             yi\t以\t40\n\
             buke\t不可\t900\n\
             keyi\t可以\t120\n\
             bukeyi\t不可以\t300\n\
             tihuan\t替换\t2000\n",
        )
    }

    #[test]
    fn splits_long_pinyin_into_words() {
        let got = segment(&dict(), "bukeyitihuan", 6);
        assert!(!got.is_empty());
        let texts: Vec<&str> = got.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.contains(&"不可以替换"), "应包含 不可以+替换，实际 {texts:?}");
    }

    #[test]
    fn prefers_whole_word() {
        let got = segment(&dict(), "bukeyi", 5);
        assert_eq!(got[0].text, "不可以");
        assert_eq!(got[0].words, vec!["不可以".to_string()]);
    }

    #[test]
    fn offers_decompositions_too() {
        let got = segment(&dict(), "bukeyitihuan", 8);
        let decompositions: Vec<Vec<String>> = got.iter().map(|s| s.words.clone()).collect();
        assert!(
            decompositions.iter().any(|w| w == &vec!["不可以".to_string(), "替换".to_string()]),
            "应有 不可以+替换，实际 {decompositions:?}"
        );
    }

    #[test]
    fn unknown_letters_yield_nothing() {
        assert!(segment(&dict(), "zzzz", 3).is_empty());
    }

    #[test]
    fn respects_k_limit() {
        assert!(segment(&dict(), "bukeyitihuan", 2).len() <= 2);
    }

    #[test]
    fn bigram_boosts_segmentation_ranking() {
        let mut userdb = crate::userdb::UserDb::default();
        userdb.record_commit("buke", "不可", None);
        userdb.record_commit("tihuan", "替换", Some("不可"));

        let without = segment(&dict(), "buketihuan", 5);
        let with = segment_with_userdb(&dict(), Some(&userdb), "buketihuan", 5);

        assert!(!without.is_empty());
        assert!(!with.is_empty());
        // 有 bigram 关联的切分成本应降低
        let cost_without = without.iter().find(|s| s.text == "不可替换").map(|s| s.cost).unwrap();
        let cost_with = with.iter().find(|s| s.text == "不可替换").map(|s| s.cost).unwrap();
        assert!(cost_with < cost_without, "共现二元应降低切分代价");
    }
}
