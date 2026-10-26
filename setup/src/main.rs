//! MC-TypeEnglish 单文件安装包 `MC-TypeEnglish-Setup.exe`。
//!
//! 设计取舍：**不自己重写一遍安装逻辑**，而是把已经跑通的 `Install.ps1` 和全套二进制
//! 内嵌进 exe，运行时释放到临时目录再调用它。这样：
//!
//! * 安装行为只有一份实现（脚本里那些哈希比对、占用跳过、ctfmon 刷新都是踩过坑的）；
//! * 用户只需要一个文件，双击即装（自动 UAC 提权）。
//!
//! 用法：
//! ```text
//!   MC-TypeEnglish-Setup.exe                  正常安装（弹 UAC）
//!   MC-TypeEnglish-Setup.exe --extract <目录>  只释放、不安装（自检用，不需要管理员）
//! ```

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, SW_SHOWNORMAL,
};

/// 内嵌的安装包内容：(相对路径, 内容)
const PAYLOAD: &[(&str, &[u8])] = &[
    (
        "mcte.exe",
        include_bytes!("../../dist/MC-TypeEnglish/mcte.exe"),
    ),
    (
        "mcte_tsf.dll",
        include_bytes!("../../dist/MC-TypeEnglish/mcte_tsf.dll"),
    ),
    (
        "mcte_tsf_x86.dll",
        include_bytes!("../../dist/MC-TypeEnglish/mcte_tsf_x86.dll"),
    ),
    (
        "mcte.ico",
        include_bytes!("../../dist/MC-TypeEnglish/mcte.ico"),
    ),
    (
        "data/dict.txt",
        include_bytes!("../../dist/MC-TypeEnglish/data/dict.txt"),
    ),
    (
        "data/dict_en.tsv",
        include_bytes!("../../dist/MC-TypeEnglish/data/dict_en.tsv"),
    ),
    (
        "data/en2zh.tsv",
        include_bytes!("../../dist/MC-TypeEnglish/data/en2zh.tsv"),
    ),
    (
        "data/glossary.tsv",
        include_bytes!("../../dist/MC-TypeEnglish/data/glossary.tsv"),
    ),
    (
        "tools/mcte-tsf-selftest.exe",
        include_bytes!("../../dist/MC-TypeEnglish/tools/mcte-tsf-selftest.exe"),
    ),
    (
        "Install.ps1",
        include_bytes!("../../dist/MC-TypeEnglish/Install.ps1"),
    ),
    (
        "Uninstall.ps1",
        include_bytes!("../../dist/MC-TypeEnglish/Uninstall.ps1"),
    ),
    (
        "README.md",
        include_bytes!("../../dist/MC-TypeEnglish/README.md"),
    ),
    (
        "LICENSE",
        include_bytes!("../../dist/MC-TypeEnglish/LICENSE"),
    ),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 自检模式：只释放，不安装、不需要管理员
    if let Some(index) = args.iter().position(|a| a == "--extract") {
        let target = args
            .get(index + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("mcte-setup-extract"));
        match extract(&target) {
            Ok(count) => {
                println!("已释放 {count} 个文件到 {}", target.display());
            }
            Err(err) => {
                eprintln!("释放失败: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if !is_elevated() {
        // 自提权：以管理员身份重新启动自己
        if let Err(err) = relaunch_elevated(&args) {
            message(&format!("需要管理员权限来注册输入法，但提权失败：\n{err}"), true);
            std::process::exit(1);
        }
        return;
    }

    let work = std::env::temp_dir().join(format!("mcte-setup-{}", std::process::id()));
    if let Err(err) = extract(&work) {
        message(&format!("释放安装文件失败：\n{err}"), true);
        std::process::exit(1);
    }

    let script = work.join("Install.ps1");
    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script.to_string_lossy(),
        ])
        .current_dir(&work)
        .status();

    let _ = std::fs::remove_dir_all(&work);

    match status {
        Ok(code) if code.success() => message(
            "安装完成。\n\n\
             用法：\n\
             · 任意程序里按 Win+Space 切到「MC-TypeEnglish」\n\
             · 终端里会自动用「终端模式」（只接管终端窗口）\n\
             · 输入拼音后：1=中文，再按 1 逐层切译文，空格上屏，回车=原样拼音\n\n\
             卸载：设置 → 应用 → 安装的应用 → MC-TypeEnglish",
            false,
        ),
        Ok(code) => message(
            &format!("安装脚本返回失败（退出码 {code:?}）。\n请查看 %TEMP% 下的日志，或用 install.bat 手动安装。"),
            true,
        ),
        Err(err) => message(&format!("无法运行安装脚本：{err}"), true),
    }
}

/// 把内嵌的负载写到 `target`。
fn extract(target: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(target)?;
    for (name, bytes) in PAYLOAD {
        let path = target.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
    }
    Ok(PAYLOAD.len())
}

fn is_elevated() -> bool {
    // 用 `net session` 这种需要管理员的命令来探测，简单且不引入额外依赖
    Command::new("net")
        .args(["session"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn relaunch_elevated(args: &[String]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_w: Vec<u16> = exe.to_string_lossy().encode_utf16().chain([0]).collect();
    let params: Vec<u16> = if args.is_empty() {
        vec![0]
    } else {
        args.join(" ").encode_utf16().chain([0]).collect()
    };
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(exe_w.as_ptr()),
            PCWSTR(params.as_ptr()),
            None,
            SW_SHOWNORMAL,
        )
    };
    if (result.0 as isize) > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecuteW(runas) 返回 {}", result.0 as isize))
    }
}

fn message(text: &str, is_error: bool) {
    let text_w: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let title: Vec<u16> = "MC-TypeEnglish 安装器".encode_utf16().chain([0]).collect();
    unsafe {
        MessageBoxW(
            Some(HWND::default()),
            PCWSTR(text_w.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | if is_error { MB_ICONERROR } else { MB_ICONINFORMATION },
        );
    }
    let _ = LPARAM(0);
    let _ = std::ptr::null::<c_void>();
}
