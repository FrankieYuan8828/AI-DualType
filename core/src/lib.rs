//! MC-TypeEnglish core — 纯逻辑 + 候选窗，供两种外壳复用：
//!
//! * `mcte`（app/）：WH_KEYBOARD_LL 钩子版，免管理员即可用；
//! * `mcte-tsf`（tsf/）：真正的 TSF 文本服务，出现在 Win+Space 里。
//!
//! 两个外壳共享同一套引擎与词库，学习结果写在同一个用户词库里，切换外壳不丢数据。

pub mod ai;
pub mod caret;
pub mod config;
pub mod dict;
pub mod engine;
pub mod glossary;
pub mod inject;
pub mod keymap;
pub mod logging;
pub mod mt;
pub mod packdict;
pub mod particle;
pub mod segment;
pub mod settings;
pub mod userdb;
pub mod version;
pub mod window;

pub use dict::Dict;
pub use engine::{Engine, Key, Outcome, Row};
pub use glossary::Glossary;
pub use packdict::PackDict;
pub use userdb::UserDb;
