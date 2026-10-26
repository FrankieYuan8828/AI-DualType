//! Optional AI completion for candidates the local glossary does not cover.
//!
//! The request shape mirrors TypeAnything's `typeanything_processor.cc`: any
//! OpenAI Chat Completions compatible endpoint (DeepSeek by default, OpenAI /
//! Moonshot / Ollama / self-hosted vLLM all work by editing `config.json`).
//!
//! The important difference is *when* it runs: TypeAnything blocks Enter and
//! rewrites already-committed text; here the request is fired on a background
//! thread while the user is still composing, and the answer is merged into the
//! candidate list (`1.4 …`) as soon as it arrives. Nothing ever blocks typing.

use std::time::Duration;

use serde_json::{json, Value};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub lang: String,
    pub text: String,
}

/// Ask the model for `word` translated into every configured target.
///
/// Returns one suggestion per language that came back filled. Errors are
/// human-readable and surfaced in the log rather than the UI, because a failed
/// AI call must never break typing.
pub fn translate(cfg: &Config, word: &str) -> Result<Vec<Suggestion>, String> {
    let ai = &cfg.ai;
    if !ai.enabled {
        return Err("AI 未启用".to_string());
    }
    if ai.api_key.trim().is_empty() {
        return Err("未配置 api_key".to_string());
    }
    if word.trim().is_empty() {
        return Ok(Vec::new());
    }

    let system = build_system_prompt(&cfg.targets);
    let url = format!("https://{}{}", ai.host.trim_end_matches('/'), ai.path);
    let payload = json!({
        "model": ai.model,
        "temperature": ai.temperature,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": word }
        ]
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(ai.timeout_secs.max(1)))
        .build();

    let response = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", ai.api_key))
        .send_json(payload)
        .map_err(|e| describe_ureq_error(&e))?;

    let body: Value = response
        .into_json()
        .map_err(|e| format!("响应不是 JSON: {e}"))?;

    let content = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| "响应里没有 choices[0].message.content".to_string())?;

    parse_answer(content, &cfg.targets)
}

fn build_system_prompt(targets: &[String]) -> String {
    format!(
        "You are a glossary. Translate the user's Chinese word or short phrase into: {}.\n\
         Rules: keep it to a word or short phrase, no explanation, no quotes, no markdown.\n\
         Reply with a single JSON object whose keys are exactly {} and whose values are the \
         translations. Example: {{\"{}\":\"...\"}}",
        targets.join(", "),
        targets.join(", "),
        targets.first().cloned().unwrap_or_else(|| "de".to_string())
    )
}

/// Parse the model answer into `(lang, text)` pairs, tolerant of code fences,
/// stray prose and `de: hallo` line formats.
pub fn parse_answer(content: &str, targets: &[String]) -> Result<Vec<Suggestion>, String> {
    let trimmed = content.trim();
    let json_slice = extract_json_object(trimmed);
    if let Some(slice) = json_slice {
        if let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(&slice) {
            let mut out = Vec::new();
            for target in targets {
                if let Some(value) = map.get(target).and_then(Value::as_str) {
                    let text = clean(value);
                    if !text.is_empty() {
                        out.push(Suggestion {
                            lang: crate::glossary::display_lang(target),
                            text,
                        });
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }
    }

    // Fallback: `de: hallo` / `德语：hallo` lines.
    let mut out = Vec::new();
    for line in trimmed.lines() {
        let (Some((key, value)), true) = (line.split_once(':').or_else(|| line.split_once('：')), true)
        else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let value = clean(value);
        if value.is_empty() {
            continue;
        }
        if let Some(target) = targets.iter().find(|t| *t == key || crate::glossary::display_lang(t) == key) {
            out.push(Suggestion {
                lang: crate::glossary::display_lang(target),
                text: value,
            });
        }
    }

    if out.is_empty() {
        Err(format!("无法解析模型输出: {trimmed}"))
    } else {
        Ok(out)
    }
}

fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(text[start..=end].to_string())
}

fn clean(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| {
            matches!(
                c,
                '"' | '\'' | '“' | '”' | '„' | '‚' | '‘' | '’' | '«' | '»'
            )
        })
        .trim_end_matches(['。', '.', '，', ','])
        .trim()
        .to_string()
}

/// 生僻词兜底：把无调拼音转成中文候选（「自动迭代」的入口）。
///
/// 词库只有 14 万条常用词，生僻词/新词注定查不到。这里的返回值会写进
/// `UserDb::spellings`，于是下一次**完全离线**也能出词 —— 越用越全，不需要重装词库。
pub fn spell(cfg: &Config, composition: &str) -> Result<Vec<String>, String> {
    let ai = &cfg.ai;
    if !ai.enabled {
        return Err("AI 未启用".to_string());
    }
    if ai.api_key.trim().is_empty() {
        return Err("未配置 api_key".to_string());
    }
    if composition.trim().is_empty() {
        return Ok(Vec::new());
    }

    let system = "You convert toneless Hanyu Pinyin into Simplified Chinese words. \
                  The user types pinyin with syllables concatenated (e.g. 'nihao' = ni+hao, \
                  'zhuanyebankuai' = zhuan+ye+ban+kuai). \
                  Reply with a JSON array of 3 candidate words, most likely first. Example: \
                  [\"你好\",\"拟好\",\"你号\"]. Output the JSON array only, no prose, no code fence.";

    let content = post_chat(cfg, system, composition)?;
    parse_word_list(&content)
}

/// 解析模型返回的候选词列表：优先 JSON 数组，退化到逐行/逗号分隔。
pub fn parse_word_list(content: &str) -> Result<Vec<String>, String> {
    let trimmed = content.trim();
    let slice = trimmed
        .find('[')
        .and_then(|start| trimmed.rfind(']').map(|end| (start, end)))
        .filter(|(start, end)| end > start)
        .map(|(start, end)| &trimmed[start..=end]);

    if let Some(slice) = slice {
        if let Ok(words) = serde_json::from_str::<Vec<String>>(slice) {
            let cleaned: Vec<String> = words
                .into_iter()
                .map(|w| clean(&w))
                .filter(|w| !w.is_empty())
                .collect();
            if !cleaned.is_empty() {
                return Ok(cleaned);
            }
        }
    }

    let fallback: Vec<String> = trimmed
        .lines()
        .flat_map(|line| line.split([',', '，', '\t']))
        .map(clean)
        .filter(|w| !w.is_empty() && !w.starts_with('[') && !w.starts_with('`'))
        .collect();
    if fallback.is_empty() {
        Err(format!("无法解析候选词: {trimmed}"))
    } else {
        Ok(fallback)
    }
}

fn post_chat(cfg: &Config, system: &str, user: &str) -> Result<String, String> {
    let ai = &cfg.ai;
    let url = format!("https://{}{}", ai.host.trim_end_matches('/'), ai.path);
    let payload = json!({
        "model": ai.model,
        "temperature": ai.temperature,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(ai.timeout_secs.max(1)))
        .build();

    let response = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", ai.api_key))
        .send_json(payload)
        .map_err(|e| describe_ureq_error(&e))?;

    let body: Value = response
        .into_json()
        .map_err(|e| format!("响应不是 JSON: {e}"))?;

    body.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| "响应里没有 choices[0].message.content".to_string())
}

fn describe_ureq_error(err: &ureq::Error) -> String {    match err {
        ureq::Error::Status(code, response) => {
            let body = response.status_text().to_string();
            match code {
                401 | 403 => format!("HTTP {code}（API key 无效或未授权）: {body}"),
                402 => format!("HTTP 402（额度不足）: {body}"),
                429 => format!("HTTP 429（请求过于频繁）: {body}"),
                other => format!("HTTP {other}: {body}"),
            }
        }
        ureq::Error::Transport(t) => format!("网络错误: {t}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets() -> Vec<String> {
        vec!["de".to_string(), "en".to_string()]
    }

    #[test]
    fn parses_plain_json() {
        let got = parse_answer(r#"{"de":"hallo","en":"hello"}"#, &targets()).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].lang, "德语");
        assert_eq!(got[0].text, "hallo");
    }

    #[test]
    fn parses_json_inside_code_fence() {
        let raw = "```json\n{\"de\": \"hallo\", \"en\": \"hi\"}\n```";
        let got = parse_answer(raw, &targets()).unwrap();
        assert_eq!(got[1].text, "hi");
    }

    #[test]
    fn parses_line_fallback() {
        let got = parse_answer("de: hallo\nen: hello", &targets()).unwrap();
        assert_eq!(got[0].text, "hallo");
    }

    #[test]
    fn ignores_unknown_languages() {
        let got = parse_answer(r#"{"fr":"bonjour"}"#, &targets());
        assert!(got.is_err(), "no configured target present -> error");
    }

    #[test]
    fn strips_trailing_punctuation_and_quotes() {
        let got = parse_answer(r#"{"de":"„hallo“","en":"hello."}"#, &targets()).unwrap();
        assert_eq!(got[0].text.trim(), "hallo");
        assert_eq!(got[1].text, "hello");
    }

    #[test]
    fn refuses_without_key() {
        let cfg = Config::default();
        assert!(translate(&cfg, "你好").is_err());
    }
}
