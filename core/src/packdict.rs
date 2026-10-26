//! 只读的「排序字典」数据包：`key<TAB>value`，按 key 排好序。
//!
//! 为什么不用 `HashMap`：这是加载进**每个宿主进程**（记事本、Chrome、explorer…）
//! 的 TSF 组件。词库 3.4 MB + 中英对照 1.9 MB + 英中反查 0.9 MB，如果都建成 HashMap，
//! 每个进程要多吃几十 MB。这里把整文件当 `Vec<u8>` 读进来（内存≈文件大小），
//! 再一次性记下每行的偏移，用二分查找按需比较 key —— 常驻内存小一个数量级。
//!
//! 生成脚本：`tools/build_glossary.py`（数据来自 CC-CEDICT）。

use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone)]
pub struct PackDict {
    data: Vec<u8>,
    /// (key 起点, key 长度, value 起点)；value 一直延伸到行尾。
    lines: Vec<(u32, u32, u32)>,
}

impl std::fmt::Debug for PackDict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不打印 5 MB 内容
        f.debug_struct("PackDict").field("entries", &self.lines.len()).finish()
    }
}

impl PackDict {
    pub fn load(path: &Path) -> io::Result<Self> {
        let data = fs::read(path)?;
        Ok(Self::from_bytes(data))
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        let mut lines = Vec::with_capacity(data.len() / 24);
        let mut line_start = 0usize;
        while line_start < data.len() {
            let line_end = match data[line_start..].iter().position(|b| *b == b'\n') {
                Some(offset) => line_start + offset,
                None => data.len(),
            };
            if let Some(tab) = data[line_start..line_end].iter().position(|b| *b == b'\t') {
                let key_start = line_start;
                let key_len = tab;
                let value_start = line_start + tab + 1;
                if key_len > 0 {
                    lines.push((
                        key_start as u32,
                        key_len as u32,
                        value_start as u32,
                    ));
                }
            }
            line_start = line_end + 1;
        }
        Self { data, lines }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn key_at(&self, index: usize) -> &str {
        let (start, len, _) = self.lines[index];
        let slice = &self.data[start as usize..(start + len) as usize];
        std::str::from_utf8(slice).unwrap_or("")
    }

    fn value_at(&self, index: usize) -> &str {
        let (_, _, value_start) = self.lines[index];
        let start = value_start as usize;
        let end = self.data[start..]
            .iter()
            .position(|b| *b == b'\n')
            .map(|offset| start + offset)
            .unwrap_or(self.data.len());
        std::str::from_utf8(&self.data[start..end])
            .unwrap_or("")
            .trim_end_matches('\r')
    }

    /// 精确查找（key 已排序，二分）。
    pub fn get(&self, key: &str) -> Option<&str> {
        let mut low = 0usize;
        let mut high = self.lines.len();
        while low < high {
            let mid = (low + high) / 2;
            match self.key_at(mid).cmp(key) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => return Some(self.value_at(mid)),
            }
        }
        None
    }

    /// 逗号/分号分隔的多个值，例如 `hello;hi`。
    pub fn get_list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(value) => value
                .split([';', ','])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect(),
            None => Vec::new(),
        }
    }

    /// 前缀查找：找到所有以 `prefix` 开头的 (key, value) 对，最多 `limit` 条。
    pub fn prefix_matches<'a>(&'a self, prefix: &str, limit: usize) -> Vec<(&'a str, &'a str)> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let prefix_lower = prefix.to_ascii_lowercase();
        let mut low = 0usize;
        let mut high = self.lines.len();
        while low < high {
            let mid = (low + high) / 2;
            if self.key_at(mid) < prefix_lower.as_str() {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let mut out = Vec::with_capacity(limit);
        for idx in low..self.lines.len() {
            let key = self.key_at(idx);
            if !key.starts_with(&prefix_lower) {
                break;
            }
            out.push((key, self.value_at(idx)));
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PackDict {
        PackDict::from_bytes(
            "apple\t苹果\nbanana\t香蕉;芭蕉\nhello\t你好\n"
                .as_bytes()
                .to_vec(),
        )
    }

    #[test]
    fn finds_exact_keys() {
        let pack = sample();
        assert_eq!(pack.get("hello"), Some("你好"));
        assert_eq!(pack.get("banana"), Some("香蕉;芭蕉"));
    }

    #[test]
    fn missing_key_is_none() {
        let pack = sample();
        assert_eq!(pack.get("zebra"), None);
        assert!(pack.get_list("zebra").is_empty());
    }

    #[test]
    fn splits_lists() {
        let pack = sample();
        assert_eq!(pack.get_list("banana"), vec!["香蕉", "芭蕉"]);
    }

    #[test]
    fn handles_trailing_newline_and_crlf() {
        let pack = PackDict::from_bytes("a\t1\r\nb\t2\r\n".as_bytes().to_vec());
        assert_eq!(pack.len(), 2);
        assert_eq!(pack.get("b"), Some("2"));
    }

    #[test]
    fn ignores_blank_and_malformed_lines() {
        let pack = PackDict::from_bytes("\n\na\t1\nbroken\nb\t2\n".as_bytes().to_vec());
        assert_eq!(pack.len(), 2);
    }

    #[test]
    fn prefix_matches_finds_words() {
        let pack = PackDict::from_bytes(
            "app\t应用\napple\t苹果\napplication\t申请\nbanana\t香蕉\n"
                .as_bytes()
                .to_vec(),
        );
        let matches = pack.prefix_matches("appl", 5);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].0, "apple");
        assert_eq!(matches[1].0, "application");
    }
}
