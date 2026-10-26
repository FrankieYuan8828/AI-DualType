//! MC-TypeEnglish · 钩子版输入法（免管理员、不需要注册 TSF）。
//!
//! 两种外壳共用 `mcte_core`：
//! * 本程序：`WH_KEYBOARD_LL` + 自绘候选窗，装上就能用；
//! * `mcte_tsf.dll`：真正的 TSF 文本服务，出现在 Win+Space 里，且能输入到提权窗口。

// 常驻运行时不要弹出黑框；`--probe` 会在启动时主动 attach 到父控制台。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod hook;
mod probe;
mod tray;

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let head = args.first().map(String::as_str);
    // 常驻模式（无参数 / --terminal-only）不要占着启动它的终端控制台：
    // 控制台光标要靠 AttachConsole 去读，自己已经挂着一个就挂不上别的了。
    if !matches!(head, None | Some("--terminal-only")) {
        attach_parent_console();
    }
    if args.iter().any(|a| a == "--trace") {
        mcte_core::caret::set_trace(true);
        mcte_core::window::set_trace(true);
    }

    match args.first().map(String::as_str) {
        Some("--probe") => {
            let input = args.get(1).cloned().unwrap_or_else(|| "nihao".to_string());
            let options = probe::Options::from_args(&args[2..]);
            if let Err(err) = probe::run(&input, options) {
                eprintln!("probe failed: {err}");
                std::process::exit(1);
            }
        }
        Some("--learn") => {
            // mcte --learn <拼音> <词> [次数]  —— 演示/验证自学习与联想排序
            let pinyin = args.get(1).cloned().unwrap_or_default();
            let word = args.get(2).cloned().unwrap_or_default();
            let times: u32 = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(5);
            if let Err(err) = probe::learn(&pinyin, &word, times) {
                eprintln!("learn failed: {err}");
                std::process::exit(1);
            }
        }
        Some("--eval") => {
            let path = args.get(1).cloned().unwrap_or_else(|| "tools\\eval_cases.tsv".to_string());
            if let Err(err) = probe::eval(std::path::Path::new(&path)) {
                eprintln!("eval failed: {err}");
                std::process::exit(1);
            }
        }
        Some("--selftest") => {
            if let Err(err) = probe::selftest() {
                eprintln!("selftest failed: {err}");
                std::process::exit(1);
            }
        }
        Some("--translate") => {
            // mcte --translate <文本> [目标语言...]  —— 验证免费网页翻译是否可用
            let text = args.get(1).cloned().unwrap_or_default();
            let targets: Vec<String> = if args.len() > 2 {
                args[2..].to_vec()
            } else {
                vec!["en".to_string(), "de".to_string(), "ja".to_string()]
            };
            for target in &targets {
                match mcte_core::mt::translate(&text, target) {
                    Ok(value) => println!("{target}\t{value}"),
                    Err(err) => println!("{target}\t<失败: {err}>"),
                }
            }
        }
        Some("--demo") => {
            let input = args.get(1).cloned().unwrap_or_else(|| "nihao".to_string());
            if let Err(err) = app::run_demo(&input) {
                eprintln!("demo failed: {err}");
                std::process::exit(1);
            }
        }
        // 终端模式：只接管终端窗口的按键（终端宿主不加载 TSF 组件）
        Some("--terminal-only") => {
            if let Err(err) = app::run() {
                eprintln!("fatal: {err}");
                std::process::exit(1);
            }
        }
        Some("--help") | Some("-h") => print_help(),
        Some("--version") | Some("-V") => println!("{}", mcte_core::version::banner()),
        Some(other) => {
            eprintln!("unknown argument: {other}");
            print_help();
            std::process::exit(2);
        }
        None => {
            if let Err(err) = app::run() {
                eprintln!("fatal: {err}");
                std::process::exit(1);
            }
        }
    }
}

fn print_help() {
    let exe = std::env::current_exe()
        .map(PathBuf::from)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "mcte.exe".to_string());
    println!(
        "{} — 拼音 + 层级翻译候选输入法（钩子版）

用法:
  {exe}                        常驻运行（托盘图标 + 全局键盘钩子）
  {exe} --probe <拼音>          打印候选列表（可加 --commit 1.1 / --userdb <文件>）
  {exe} --learn <拼音> <词> [n]  模拟上屏 n 次，演示自学习与联想排序
  {exe} --demo  <拼音>          打开候选窗预览 10 秒（截图验证用）
  {exe} --help                  显示本帮助
  {exe} --version               显示版本

常用快捷键:
  Ctrl+Alt+P                   切换 中文/英文 模式（可在 config.json 改）
  1 / 1.1 / 2                  数字键选择层级候选；空格/回车上屏；Esc 取消

想在 Win+Space 里使用、并输入到管理员权限的窗口，请安装 TSF 版本：
  以管理员身份运行 dist\\MC-TypeEnglish\\Install.ps1
",
        mcte_core::version::banner()
    );
}

/// Release builds run as a GUI subsystem binary (no console flash). CLI modes
/// still want their output, so borrow the parent console when there is one.
fn attach_parent_console() {
    #[cfg(not(debug_assertions))]
    unsafe {
        use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows::Win32::System::Console::{
            AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
            STD_OUTPUT_HANDLE,
        };
        // 已经继承了可用的标准输出（管道或重定向文件）时不要去抢父进程的控制台：
        // AttachConsole 会把标准句柄换成 CONOUT$，反而把输出写到看不见的地方。
        let inherited = [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE].iter().any(|kind| {
            match GetStdHandle(*kind) {
                Ok(handle) => !handle.is_invalid() && handle != INVALID_HANDLE_VALUE,
                Err(_) => false,
            }
        });
        if !inherited {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
