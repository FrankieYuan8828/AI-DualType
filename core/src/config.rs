//! User configuration, stored as JSON next to the user's AppData profile.
//!
//! Rationale: JSON + serde keeps the file hand-editable (no GUI needed for v1)
//! while still round-tripping unknown future keys through `extra`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const APP_DIR: &str = "MC-TypeEnglish";
pub const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Master switch. When false the IME stays fully offline.
    pub enabled: bool,
    pub host: String,
    pub path: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f32,
    pub timeout_secs: u64,
    /// How many of the top Chinese candidates receive AI sub-candidates.
    pub max_parents: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "api.deepseek.com".to_string(),
            path: "/v1/chat/completions".to_string(),
            model: "deepseek-chat".to_string(),
            api_key: String::new(),
            temperature: 0.2,
            timeout_secs: 8,
            max_parents: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Translation targets shown as `1.1`, `1.2`, ... sub-candidates.
    /// Language codes are free-form: `de`, `en`, `ja`, `fr`, ... or any
    /// natural-language description the AI is asked to render.
    pub targets: Vec<String>,
    /// Start in ABC (ASCII passthrough) mode.
    pub start_in_ascii_mode: bool,
    /// Global toggle hotkey, e.g. `Ctrl+Alt+P`.
    pub toggle_hotkey: String,
    /// 候选总数（分页前的池子大小）。
    pub max_parents: usize,
    /// 每页显示多少组（上方中文 + 下方英文算一组）。
    pub page_size: usize,
    /// Translation sub-candidates per parent.
    pub max_children: usize,
    /// Also translate dictionaries' abbreviations / prefix matches.
    pub prefix_search: bool,
    /// 记住用户选过的词：词频排序 + 上下文联想 + 生僻词自动编码。
    pub learn_enabled: bool,
    /// 词库完全查不到时，是否让 AI 兜底给出候选并落进用户词库（生僻词自动迭代）。
    pub ai_spell_fallback: bool,
    /// 用公开的免费网页翻译接口补译文（不需要 API key）。
    /// AI 只是更准的可选项；关掉它也不影响翻译。
    pub web_translate: bool,
    /// 除英文之外「再跟随一种语言」，也可以不跟。英文永远跟随。
    ///
    /// 这是候选窗右上角那个设置按钮的唯一作用：用户只在这几个语言里挑一个，
    /// 挑中的语言译文出现在英文那一行下面。
    pub extra_language: Option<String>,
    pub ai: AiConfig,
}

/// 设置里可选的语言（代码, 显示名）。英文不在此列——它固定跟随。
pub const EXTRA_LANGUAGES: [(&str, &str); 8] = [
    ("de", "德语"),
    ("fr", "法语"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("ru", "俄语"),
    ("es", "西班牙语"),
    ("it", "意大利语"),
    ("pt", "葡萄牙语"),
];

impl Default for Config {
    fn default() -> Self {
        Self {
            // German first on purpose: `nihao` -> `1.1 hallo`, matching the
            // spec this project was built against.
            // 英文放第一位：候选卡片「下层」显示的就是英文，和 1.1 的选择一致
            targets: vec!["en".to_string(), "de".to_string(), "ja".to_string()],
            start_in_ascii_mode: false,
            toggle_hotkey: "Ctrl+Alt+P".to_string(),
            max_parents: 21,
            page_size: 7,
            max_children: 3,
            prefix_search: true,
            learn_enabled: true,
            ai_spell_fallback: true,
            web_translate: true,
            extra_language: None,
            ai: AiConfig::default(),
        }
    }
}

impl Config {
    pub fn load_or_create(dir: &Path) -> Self {
        let path = dir.join(CONFIG_FILE);
        if let Ok(raw) = fs::read_to_string(&path) {
            match serde_json::from_str::<Config>(&raw) {
                Ok(cfg) => return cfg.sanitised(),
                Err(err) => {
                    crate::logging::log(&format!(
                        "config: {} is invalid ({err}), falling back to defaults",
                        path.display()
                    ));
                }
            }
        }
        let cfg = Config::default();
        let _ = cfg.save(dir);
        cfg
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dir)?;
        let raw = serde_json::to_string_pretty(self).expect("config serialises");
        fs::write(dir.join(CONFIG_FILE), raw)
    }

    /// Clamp values the UI and the engine rely on.
    fn sanitised(mut self) -> Self {
        self.page_size = self.page_size.clamp(1, 9);
        self.max_parents = self.max_parents.clamp(self.page_size, 63);
        self.max_children = self.max_children.clamp(0, 9);

        // `targets` 不再是手写列表：英文固定跟随，`extra_language` 决定要不要
        // 再跟一种。设置弹窗改的就是这一个字段。
        if let Some(extra) = self.extra_language.clone() {
            let known = EXTRA_LANGUAGES.iter().any(|(code, _)| *code == extra);
            if !known || extra == "en" {
                self.extra_language = None;
            }
        }
        let mut targets = vec!["en".to_string()];
        if let Some(extra) = &self.extra_language {
            targets.push(extra.clone());
        }
        self.targets = targets;
        // 英文 + 额外语言至少各占一行，太小的上限会让配置看起来「没生效」。
        if self.max_children < self.targets.len() {
            self.max_children = self.targets.len().max(2);
        }
        self
    }

    /// 设置弹窗选完后写回配置。
    pub fn set_extra_language(&mut self, language: Option<&str>) {
        self.extra_language = language
            .filter(|code| *code != "en")
            .map(|code| code.to_string());
        *self = self.clone().sanitised();
    }
}

/// `%APPDATA%\MC-TypeEnglish`
pub fn app_data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_DIR)
}

/// 进程内覆盖数据目录。
///
/// TSF 版是加载进宿主机进程（notepad.exe / chrome.exe …）的 DLL，
/// `current_exe()` 拿到的是宿主程序，找不到 `data\dict.txt`。
/// 所以 TSF 在激活时会用**自己 DLL 所在目录**覆盖掉它。
static DATA_DIR_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn set_data_dir(path: PathBuf) {
    let _ = DATA_DIR_OVERRIDE.set(path);
}

/// Directory holding `dict.txt`, `glossary.tsv` and the exe itself.
///
/// A `data` directory next to the executable wins; falling back to the current
/// working directory keeps `cargo run` usable during development.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = DATA_DIR_OVERRIDE.get() {
        return dir.clone();
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("data");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from("data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_within_limits() {
        let cfg = Config::default().sanitised();
        assert!(cfg.max_parents <= 63);
        assert_eq!(cfg.page_size, 7);
        assert_eq!(cfg.targets.first().map(String::as_str), Some("en"));
    }

    #[test]
    fn roundtrip_preserves_values() {
        let tmp = std::env::temp_dir().join("mcte-config-test");
        let _ = fs::remove_dir_all(&tmp);
        let mut cfg = Config::default();
        cfg.max_parents = 14;
        cfg.save(&tmp).unwrap();
        let loaded = Config::load_or_create(&tmp);
        assert_eq!(loaded.max_parents, 14);
        let _ = fs::remove_dir_all(&tmp);
    }
}
