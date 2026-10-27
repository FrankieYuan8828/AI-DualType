//! Local translation glossary: `word<TAB>lang<TAB>translation`.
//!
//! This is what makes the `1.1 hallo` sub-candidate appear instantly, without
//! waiting for a network round-trip. Users extend it by editing
//! `data/glossary.tsv`; the AI worker fills the gaps for unknown words.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::packdict::PackDict;

#[derive(Debug, Default, Clone)]
pub struct Glossary {
    /// word -> lang -> translation (first entry wins). 手写词表，优先级最高。
    entries: HashMap<String, Vec<(String, String)>>,
    /// word -> 英文释义（CC-CEDICT，5.7 万词）。手写表没有英文时用它兜底。
    en_pack: Option<std::sync::Arc<PackDict>>,
    /// 英文 -> 中文候选（CC-CEDICT 反查 + 手写表），用于「打英文出中文」。
    reverse: Option<std::sync::Arc<PackDict>>,
    /// 数据目录，用于懒加载（避免激活时就把几 MB 读进来）。
    data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translation {
    pub lang: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnglishAssociation {
    pub word: String,
    pub translations: Vec<String>,
    pub rank: u32,
    pub weight: u32,
}

impl Glossary {
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        Ok(Self::parse(&raw))
    }

    /// 从一个数据目录装载完整能力：手写词表 + 中英对照包 + 英中反查包。
    ///
    /// 两个 `.tsv` 包缺失时能力自动降级（只有手写词表），不会导致激活失败。
    pub fn from_dir(dir: &Path) -> Self {
        let mut glossary = match Self::load(&dir.join("glossary.tsv")) {
            Ok(parsed) => parsed,
            Err(_) => Self::default(),
        };
        glossary.en_pack = PackDict::load(&dir.join("dict_en.tsv"))
            .ok()
            .map(std::sync::Arc::new);
        glossary.reverse = PackDict::load(&dir.join("en2zh.tsv"))
            .ok()
            .map(std::sync::Arc::new);
        glossary.data_dir = Some(dir.to_path_buf());
        glossary
    }

    pub fn set_reverse(&mut self, pack: PackDict) {
        self.reverse = Some(std::sync::Arc::new(pack));
    }

    /// 数据包规模（词表条数, 中英词条数, 反查词条数），日志与自检用。
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.entries.len(),
            self.en_pack.as_ref().map(|p| p.len()).unwrap_or(0),
            self.reverse.as_ref().map(|p| p.len()).unwrap_or(0),
        )
    }

    /// 英文 → 中文候选（「打英文出中文」），返回 `(中文词, 词频序, 匹配权重)`。
    ///
    /// * `rank`：中文词的全局词频序（`china` 同时是合法拼音，靠它和拼音候选竞争）；
    /// * `weight`：这个英文词在该中文词释义里的位置（越靠前越能代表它），
    ///   引擎用它做加权 —— 否则 `thanks` 会被词频更高但释义排在第 3 位的「由于」抢走。
    pub fn reverse_lookup(&self, word: &str, limit: usize) -> Vec<(String, u32, u32)> {
        let Some(pack) = self.reverse.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(limit);
        for item in pack.get_list(&word.to_ascii_lowercase()) {
            let mut parts = item.rsplitn(3, ':');
            let (zh, rank, weight) = match (
                parts.next(),
                parts.next(),
                parts.next(),
            ) {
                (Some(weight), Some(rank), Some(zh)) => (
                    zh.trim(),
                    rank.trim().parse::<u32>().unwrap_or(90_000),
                    weight.trim().parse::<u32>().unwrap_or(1),
                ),
                (Some(rank), Some(zh), None) => (
                    zh.trim(),
                    rank.trim().parse::<u32>().unwrap_or(90_000),
                    1,
                ),
                (Some(zh), _, _) => (zh.trim(), 90_000, 1),
                _ => continue,
            };
            if zh.is_empty() {
                continue;
            }
            out.push((zh.to_string(), rank, weight));
            if out.len() >= limit {
                break;
            }
        }
        out
    }

    /// 英文词查中文译文（输入英文单词直接查中文候选释义）
    pub fn english_to_chinese(&self, word: &str, limit: usize) -> Vec<String> {
        let Some(pack) = self.reverse.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in pack.get_list(&word.to_ascii_lowercase()) {
            let mut parts = item.rsplitn(3, ':');
            let zh = match (parts.next(), parts.next(), parts.next()) {
                (Some(_w), Some(_r), Some(zh)) => zh.trim(),
                (Some(_r), Some(zh), None) => zh.trim(),
                (Some(zh), _, _) => zh.trim(),
                _ => continue,
            };
            if !zh.is_empty() && !out.contains(&zh.to_string()) {
                out.push(zh.to_string());
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    /// 英文联想词模式：输入英文前缀，返回推荐的英文联想候选及其对应的中文释义。
    pub fn english_associations(&self, prefix: &str, limit: usize) -> Vec<EnglishAssociation> {
        let Some(pack) = self.reverse.as_ref() else {
            return Vec::new();
        };
        let prefix_lower = prefix.to_ascii_lowercase();
        let scan_limit = match prefix_lower.len() {
            1 => 1500,
            2 => 500,
            _ => (limit * 8).max(100),
        };
        let matches = pack.prefix_matches(&prefix_lower, scan_limit);
        let mut results = Vec::new();
        for (key, val) in matches {
            let mut zh_list = Vec::new();
            let mut best_rank = 90_000u32;
            let mut best_weight = 1u32;
            for item in val.split([';', ',']) {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                let mut parts = item.rsplitn(3, ':');
                let (zh, rank, weight) = match (parts.next(), parts.next(), parts.next()) {
                    (Some(w), Some(r), Some(z)) => (
                        z.trim(),
                        r.trim().parse::<u32>().unwrap_or(90_000),
                        w.trim().parse::<u32>().unwrap_or(1),
                    ),
                    (Some(r), Some(z), None) => (
                        z.trim(),
                        r.trim().parse::<u32>().unwrap_or(90_000),
                        1,
                    ),
                    (Some(z), _, _) => (z.trim(), 90_000, 1),
                    _ => continue,
                };
                if !zh.is_empty() && !zh_list.contains(&zh.to_string()) {
                    zh_list.push(zh.to_string());
                    if rank < best_rank {
                        best_rank = rank;
                    }
                    if weight > best_weight {
                        best_weight = weight;
                    }
                }
                if zh_list.len() >= 3 {
                    break;
                }
            }
            if !zh_list.is_empty() {
                results.push(EnglishAssociation {
                    word: key.to_string(),
                    translations: zh_list,
                    rank: best_rank,
                    weight: best_weight,
                });
            }
        }

        results.sort_by(|a, b| {
            let a_exact = a.word == prefix_lower;
            let b_exact = b.word == prefix_lower;
            if a_exact != b_exact {
                return b_exact.cmp(&a_exact);
            }
            let a_single = !a.word.contains([' ', '-']);
            let b_single = !b.word.contains([' ', '-']);
            if a_single != b_single {
                return b_single.cmp(&a_single);
            }
            let a_score = (a.rank as i64) - (a.weight as i64) * 2000;
            let b_score = (b.rank as i64) - (b.weight as i64) * 2000;
            a_score.cmp(&b_score).then_with(|| a.word.len().cmp(&b.word.len()))
        });

        results.truncate(limit);
        results
    }

    pub fn parse(raw: &str) -> Self {
        let mut entries: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for line in raw.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            let (Some(word), Some(lang), Some(text)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let (word, lang, text) = (word.trim(), lang.trim(), text.trim());
            if word.is_empty() || lang.is_empty() || text.is_empty() {
                continue;
            }
            let bucket = entries.entry(word.to_string()).or_default();
            if bucket.iter().any(|(l, _)| l == lang) {
                continue;
            }
            bucket.push((lang.to_string(), text.to_string()));
        }
        Self {
            entries,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Translations for `word`, ordered exactly like `targets`.
    ///
    /// 一条译文就是一个候选行，绝不拼成 `A / B`：用户要的是连按同一个数字
    /// 逐行往下选，而不是盯着一个斜杠串猜哪个是哪个。
    pub fn translations(&self, word: &str, targets: &[String], limit: usize) -> Vec<Translation> {
        if targets.is_empty() || limit == 0 {
            return Vec::new();
        }
        let bucket = self.entries.get(word);
        let mut out = Vec::with_capacity(targets.len());
        for target in targets {
            if let Some(text) = bucket
                .and_then(|items| items.iter().find(|(lang, _)| lang == target))
                .map(|(_, text)| text.clone())
            {
                out.push(Translation {
                    lang: display_lang(target),
                    text,
                });
            } else if target == "en" {
                // 手写表没这个词，就用 CC-CEDICT 的中英对照（覆盖 5.7 万词）。
                // 一个词条常常有两三个意思，每个意思各占一行，方便直接挑。
                if let Some(value) = self.en_pack.as_ref().and_then(|pack| pack.get(word)) {
                    for gloss in value.split(';').map(str::trim).filter(|part| !part.is_empty()) {
                        out.push(Translation {
                            lang: display_lang("en"),
                            text: gloss.to_string(),
                        });
                        if out.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if out.len() >= limit {
                break;
            }
        }
        out
    }

    /// Direct lookup for a single language, used by the AI cache merge.
    #[allow(dead_code)]
    pub fn translation_for(&self, word: &str, lang: &str) -> Option<String> {
        self.entries
            .get(word)
            .and_then(|bucket| bucket.iter().find(|(l, _)| l == lang))
            .map(|(_, text)| text.clone())
    }
}

/// Human-readable language name for the candidate label suffix.
pub fn display_lang(code: &str) -> String {
    match code.to_ascii_lowercase().as_str() {
        "de" => "德语".to_string(),
        "en" => "英文".to_string(),
        "ja" => "日语".to_string(),
        "ko" => "韩语".to_string(),
        "fr" => "法语".to_string(),
        "es" => "西语".to_string(),
        "ru" => "俄语".to_string(),
        "it" => "意语".to_string(),
        "pt" => "葡语".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Glossary {
        Glossary::parse(
            "# comment line\n\
             你好\tde\thallo\n\
             你好\ten\thello\n\
             你好\tja\tこんにちは\n\
             谢谢\ten\tthanks\n\
             broken line without tabs\n",
        )
    }

    #[test]
    fn follows_target_order() {
        let g = sample();
        let targets = vec!["de".to_string(), "en".to_string()];
        let got = g.translations("你好", &targets, 5);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].text, "hallo");
        assert_eq!(got[0].lang, "德语");
        assert_eq!(got[1].text, "hello");
    }

    #[test]
    fn respects_limit() {
        let g = sample();
        let targets = vec!["de".into(), "en".into(), "ja".into()];
        assert_eq!(g.translations("你好", &targets, 1).len(), 1);
    }

    #[test]
    fn unknown_word_is_empty() {
        let g = sample();
        assert!(g.translations("不存在", &["de".to_string()], 3).is_empty());
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let g = sample();
        assert_eq!(g.len(), 2, "two distinct words expected");
    }

    #[test]
    fn english_associations_returns_prefix_candidates() {
        let mut g = sample();
        let reverse_pack = PackDict::from_bytes(
            "apple\t苹果:800:99\napplication\t应用程序:1500:90;申请:1200:80\nbanana\t香蕉:2000:90\n"
                .as_bytes()
                .to_vec(),
        );
        g.reverse = Some(std::sync::Arc::new(reverse_pack));
        let assocs = g.english_associations("appl", 5);
        assert_eq!(assocs.len(), 2);
        assert_eq!(assocs[0].word, "apple");
        assert_eq!(assocs[0].translations, vec!["苹果"]);
        assert_eq!(assocs[1].word, "application");
    }
}
