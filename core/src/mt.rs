//! 免费网页翻译：不需要任何 API key。
//!
//! 用户的原话是「直接通用的链接免费的 web 翻译返回结果即可」。这里就照这个做：
//! 先把整句/整词发到公开的翻译接口，拿到结果后缓存起来；失败再换下一个接口。
//! 全部在后台线程里跑，超时很短，绝不会卡住打字。
//!
//! 两个实现：
//!   * Google 的 `translate_a/single`（gtx 客户端参数）——支持任意目标语言；
//!   * MyMemory 的公开接口——只覆盖中英，作为兜底。
//!
//! 公开接口的额度与连通性都不由我们控制，所以这里的原则是：尽力而为、失败即放弃、
//! 结果缓存，绝不因为网络问题影响本地词库的候选。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::glossary::display_lang;

/// 网络请求超时：短一点，宁可没有译文也不要让人等。
const TIMEOUT: Duration = Duration::from_secs(4);
/// 缓存条目上限与保留时长。
const CACHE_LIMIT: usize = 1024;
const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

struct Entry {
    text: String,
    at: Instant,
}

fn cache() -> &'static Mutex<HashMap<(String, String), Entry>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<(String, String), Entry>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 把 `text` 翻成 `target`（`en` / `de` / `ja` …）。
pub fn translate(text: &str, target: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(String::new());
    }
    let key = (text.to_string(), target.to_string());
    if let Ok(guard) = cache().lock() {
        if let Some(entry) = guard.get(&key) {
            if entry.at.elapsed() < CACHE_TTL {
                return Ok(entry.text.clone());
            }
        }
    }

    let mut errors: Vec<String> = Vec::new();
    for (name, provider) in [
        ("google-dict", google_dict as fn(&str, &str) -> Result<String, String>),
        ("google-gtx", google_gtx),
        ("mymemory", mymemory),
    ] {
        match provider(text, target) {
            Ok(value) if !value.trim().is_empty() => {
                remember(key, value.clone());
                return Ok(value);
            }
            Ok(_) => errors.push(format!("{name}: 空结果")),
            Err(err) => errors.push(format!("{name}: {err}")),
        }
    }
    Err(errors.join(" | "))
}

/// 一次翻成多个语言，返回 `(语言标签, 译文)`。
pub fn translate_many(text: &str, targets: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for target in targets {
        match translate(text, target) {
            Ok(value) if !value.trim().is_empty() => {
                out.push((display_lang(target), value));
            }
            _ => continue,
        }
    }
    out
}

fn remember(key: (String, String), text: String) {
    if let Ok(mut guard) = cache().lock() {
        if guard.len() >= CACHE_LIMIT {
            guard.retain(|_, entry| entry.at.elapsed() < CACHE_TTL);
            if guard.len() >= CACHE_LIMIT {
                guard.clear();
            }
        }
        guard.insert(
            key,
            Entry {
                text,
                at: Instant::now(),
            },
        );
    }
}

/// Google 的 dict-chrome-ex 端点：返回 `["译文"]`，结构最简单，实测最稳。
fn google_dict(text: &str, target: &str) -> Result<String, String> {
    let url = format!(
        "https://clients5.google.com/translate_a/t?client=dict-chrome-ex&sl=zh-CN&tl={}&q={}",
        percent_encode(target),
        percent_encode(text)
    );
    let body = get(&url)?;
    let parsed: Value = serde_json::from_str(&body).map_err(|e| format!("解析失败: {e}"))?;
    let join = |value: &Value| -> String {
        match value {
            Value::String(text) => text.clone(),
            Value::Array(items) => items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            _ => String::new(),
        }
    };
    match &parsed {
        Value::Array(items) => {
            let mut out = String::new();
            for item in items {
                // 单句时是 `["译文"]`，多句时是 `[["句子","译文"], ...]`
                let piece = match item {
                    Value::Array(pair) => pair.get(1).map(&join).unwrap_or_default(),
                    other => join(other),
                };
                out.push_str(&piece);
            }
            Ok(out.trim().to_string())
        }
        other => Ok(join(other).trim().to_string()),
    }
}

/// Google 的 gtx 端点：返回 `[[["Hello","你好",…],…],…]`，把各段拼起来。
fn google_gtx(text: &str, target: &str) -> Result<String, String> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=zh-CN&tl={}&dt=t&q={}",
        percent_encode(target),
        percent_encode(text)
    );
    let body = get(&url)?;
    let parsed: Value = serde_json::from_str(&body).map_err(|e| format!("解析失败: {e}"))?;
    let segments = parsed
        .get(0)
        .and_then(Value::as_array)
        .ok_or_else(|| "返回结构异常".to_string())?;
    let mut out = String::new();
    for segment in segments {
        if let Some(piece) = segment.get(0).and_then(Value::as_str) {
            out.push_str(piece);
        }
    }
    Ok(out.trim().to_string())
}

/// MyMemory：只支持中英，作为 Google 不通时的兜底。
fn mymemory(text: &str, target: &str) -> Result<String, String> {
    if target != "en" {
        return Err("MyMemory 只用于中英".to_string());
    }
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair=zh-CN|en",
        percent_encode(text)
    );
    let body = get(&url)?;
    let parsed: Value = serde_json::from_str(&body).map_err(|e| format!("解析失败: {e}"))?;
    parsed
        .get("responseData")
        .and_then(|data| data.get("translatedText"))
        .and_then(Value::as_str)
        .map(|text| text.trim().to_string())
        .ok_or_else(|| "返回结构异常".to_string())
}

fn get(url: &str) -> Result<String, String> {
    let response = agent().get(url).call().map_err(|e| e.to_string())?;
    response.into_string().map_err(|e| e.to_string())
}

/// 带代理的 HTTP 客户端。
///
/// 这台机器（以及国内大部分机器）都挂着 Clash 之类的系统代理，而 ureq 默认只认
/// `HTTP_PROXY` 环境变量、不会去读 Windows 的代理设置——于是浏览器里能打开的
/// 翻译接口，程序里却直连超时。这里显式把系统代理取出来交给它。
fn agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout(TIMEOUT);
    if let Some(url) = proxy_url() {
        match ureq::Proxy::new(&url) {
            Ok(proxy) => {
                crate::logging::log(&format!("mt: using proxy {url}"));
                builder = builder.proxy(proxy);
            }
            Err(err) => crate::logging::log(&format!("mt: proxy {url} invalid: {err}")),
        }
    }
    builder.build()
}

/// 依次看环境变量和 Windows 的「Internet 选项 → 连接 → 局域网设置」。
fn proxy_url() -> Option<String> {
    for name in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(with_scheme(&value));
            }
        }
    }
    let (enabled, server) = registry_proxy()?;
    if !enabled || server.trim().is_empty() {
        return None;
    }
    // `ProxyServer` 可能是 `127.0.0.1:7890`，也可能是 `http=…;https=…`
    let picked = server
        .split(';')
        .find_map(|part| {
            let (scheme, address) = part.split_once('=')?;
            if scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http") {
                Some(address.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| server.clone());
    Some(with_scheme(picked.trim()))
}

fn with_scheme(address: &str) -> String {
    if address.contains("://") {
        address.to_string()
    } else {
        format!("http://{address}")
    }
}

/// 读 `HKCU\…\Internet Settings` 里的 `ProxyEnable` / `ProxyServer`。
fn registry_proxy() -> Option<(bool, String)> {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };

    unsafe {
        let mut enabled: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let ok = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings"),
            w!("ProxyEnable"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut enabled as *mut u32 as *mut core::ffi::c_void),
            Some(&mut size),
        );
        if ok.is_err() {
            return None;
        }

        let mut buffer = [0u16; 512];
        let mut size = (buffer.len() * 2) as u32;
        let ok = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings"),
            w!("ProxyServer"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            Some(&mut size),
        );
        let server = if ok.is_ok() {
            let len = buffer.iter().position(|unit| *unit == 0).unwrap_or(0);
            String::from_utf16_lossy(&buffer[..len])
        } else {
            String::new()
        };
        Some((enabled == 1, server))
    }
}

/// 最小可用的百分号编码：只放行 URL 里安全的字符，其余按 UTF-8 字节转义。
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encoding_keeps_ascii_words() {
        assert_eq!(percent_encode("hello-world_1.0~"), "hello-world_1.0~");
    }

    #[test]
    fn percent_encoding_escapes_utf8_bytes() {
        assert_eq!(percent_encode("你好"), "%E4%BD%A0%E5%A5%BD");
    }

    #[test]
    fn percent_encoding_escapes_spaces() {
        assert_eq!(percent_encode("a b"), "a%20b");
    }
}
