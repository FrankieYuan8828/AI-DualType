//! Tiny append-only log file. The release binary runs under the Windows
//! subsystem (no console), so this is the only way to see what the IME did.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 512 * 1024;

pub fn log(message: &str) {
    let path = crate::config::app_data_dir().join("mcte.log");
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_BYTES {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::rename(&path, rotated);
        }
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "[{stamp}] {message}");
    }
}
