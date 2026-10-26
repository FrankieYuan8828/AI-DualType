//! 单点版本号，安装器、托盘菜单与 `--version` 都读这里。

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PRODUCT: &str = "MC-TypeEnglish";

/// 给用户看的完整标识，例如 `MC-TypeEnglish 1.0.0`。
pub fn banner() -> String {
    format!("{PRODUCT} {VERSION}")
}
