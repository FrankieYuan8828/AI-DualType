//! Where to put the candidate window: just under the text caret, or under the
//! mouse pointer when the focused window does not expose a caret rectangle
//! (browsers, Electron apps and some games).
//!
//! `conhost`（cmd.exe / PowerShell 的老式控制台窗口）属于最后一种：它是 GDI
//! 自绘的，`GetGUIThreadInfo` 不填 `rcCaret`，于是候选窗会掉到鼠标位置上去，
//! 在屏幕上看起来就是「按了键什么都没有」。控制台的光标位置只能从控制台 API
//! 读，所以这里先临时挂到那个控制台上读光标所在单元格，再换算成屏幕坐标。

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, GetConsoleScreenBufferInfo, GetConsoleWindow,
    GetCurrentConsoleFontEx, CONSOLE_FONT_INFOEX, CONSOLE_SCREEN_BUFFER_INFO,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    GUITHREADINFO,
};

const CONSOLE_CLASS: &str = "ConsoleWindowClass";

/// `MCTE_TRACE=1` / `--trace` 时把定位过程写进日志。
static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_trace(on: bool) {
    TRACE.store(on, std::sync::atomic::Ordering::Relaxed);
}

fn trace(message: &str) {
    if TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        crate::logging::log(&format!("caret: {message}"));
    }
}

static LAST_CARET: std::sync::Mutex<Option<(isize, POINT)>> = std::sync::Mutex::new(None);

/// Screen coordinates for the *bottom-left* corner of the caret.
pub fn caret_point() -> POINT {
    unsafe {
        let foreground = GetForegroundWindow();
        if !foreground.is_invalid() {
            if let Some(point) = gui_caret_point(foreground) {
                if let Ok(mut guard) = LAST_CARET.lock() {
                    *guard = Some((foreground.0 as isize, point));
                }
                return point;
            }
            if window_class(foreground) == CONSOLE_CLASS {
                if let Some(point) = console_caret_point(foreground) {
                    if let Ok(mut guard) = LAST_CARET.lock() {
                        *guard = Some((foreground.0 as isize, point));
                    }
                    return point;
                }
                trace("console caret unavailable, falling back to window corner");
                if let Some(point) = console_corner_point(foreground) {
                    return point;
                }
            }
            // 兜底：如果当前前台窗口之前成功读到过有效光标，优先沿用，绝不突然瞬移到鼠标位置！
            if let Ok(guard) = LAST_CARET.lock() {
                if let Some((hwnd, point)) = *guard {
                    if hwnd == foreground.0 as isize {
                        trace("reusing last valid caret for foreground window");
                        return point;
                    }
                }
            }
        }

        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_ok() {
            // Offset a little so the window does not sit under the pointer.
            return POINT {
                x: cursor.x + 8,
                y: cursor.y + 20,
            };
        }
        POINT { x: 200, y: 200 }
    }
}

/// 普通 GUI 程序的光标。控制台窗口返回全零矩形，这里按「高度为 0」判掉，
/// 免得把候选窗钉在窗口左上角。
unsafe fn gui_caret_point(foreground: HWND) -> Option<POINT> {
    let thread_id = GetWindowThreadProcessId(foreground, None);
    if thread_id == 0 {
        return None;
    }
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    if GetGUIThreadInfo(thread_id, &mut info).is_err() {
        return None;
    }
    let caret: RECT = info.rcCaret;
    if caret.bottom <= caret.top || caret.right < caret.left {
        return None;
    }
    if info.hwndCaret.is_invalid() {
        return None;
    }
    let mut point = POINT {
        x: caret.left,
        y: caret.bottom,
    };
    if !ClientToScreen(info.hwndCaret, &mut point).as_bool() {
        return None;
    }
    Some(point)
}

/// 控制台窗口：临时 AttachConsole 到控制台，读光标所在单元格。
///
/// 控制台同一时刻可能有多个客户端（cmd / powershell / python …），而窗口句柄
/// 属于 conhost，所以按名字挑出可能的客户端逐个试挂，直到 `GetConsoleWindow()`
/// 对上前台窗口为止。每次挂接都很快释放，不会影响控制台里的程序。
unsafe fn console_caret_point(target: HWND) -> Option<POINT> {
    let candidates = console_candidates();
    trace(&format!(
        "console path: candidates={:?}, already_attached={:?}",
        candidates,
        GetConsoleWindow()
    ));
    for pid in candidates {
        if let Err(err) = AttachConsole(pid) {
            trace(&format!("attach pid={pid} failed: {err}"));
            continue;
        }
        let attached = GetConsoleWindow();
        let mut result = None;
        if attached == target {
            result = read_console_caret(target);
            trace(&format!("attach pid={pid} matched, caret={result:?}"));
        } else {
            trace(&format!("attach pid={pid} other console {attached:?}"));
        }
        let free = FreeConsole();
        if free.is_err() {
            trace(&format!("free after pid={pid} failed: {:?}", free.err()));
        }
        if result.is_some() {
            return result;
        }
    }
    None
}

unsafe fn read_console_caret(window: HWND) -> Option<POINT> {
    let console = CreateFileW(
        windows::core::w!("CONOUT$"),
        windows::Win32::Foundation::GENERIC_READ.0 | windows::Win32::Foundation::GENERIC_WRITE.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        None,
    )
    .ok()?;

    let mut buffer_info = CONSOLE_SCREEN_BUFFER_INFO::default();
    let read_ok = GetConsoleScreenBufferInfo(console, &mut buffer_info).is_ok();

    let mut font = CONSOLE_FONT_INFOEX {
        cbSize: std::mem::size_of::<CONSOLE_FONT_INFOEX>() as u32,
        ..Default::default()
    };
    let font_ok = GetCurrentConsoleFontEx(console, false, &mut font).is_ok();
    let _ = windows::Win32::Foundation::CloseHandle(console);

    if !read_ok {
        return None;
    }
    // 光栅字体会把字号报成 0，按典型控制台默认值兜底。
    let cell_w = if font_ok && font.dwFontSize.X > 0 {
        font.dwFontSize.X as i32
    } else {
        8
    };
    let cell_h = if font_ok && font.dwFontSize.Y > 0 {
        font.dwFontSize.Y as i32
    } else {
        16
    };

    let mut origin = POINT { x: 0, y: 0 };
    if !ClientToScreen(window, &mut origin).as_bool() {
        return None;
    }
    let cursor = buffer_info.dwCursorPosition;
    trace(&format!(
        "cursor=({}, {}) cell={}x{} origin=({}, {})",
        cursor.X, cursor.Y, cell_w, cell_h, origin.x, origin.y
    ));
    Some(POINT {
        x: origin.x + cursor.X as i32 * cell_w,
        y: origin.y + (cursor.Y as i32 + 1) * cell_h,
    })
}

/// 读不到光标时的保底位置：控制台窗口客户区左下角（正常使用时命令提示符
/// 就在屏幕下方，比鼠标位置靠谱得多）。
unsafe fn console_corner_point(window: HWND) -> Option<POINT> {
    let mut client = RECT::default();
    if !windows::Win32::UI::WindowsAndMessaging::GetClientRect(window, &mut client).is_ok() {
        return None;
    }
    let mut origin = POINT { x: 0, y: 0 };
    if !ClientToScreen(window, &mut origin).as_bool() {
        return None;
    }
    Some(POINT {
        x: origin.x + 8,
        y: origin.y + client.bottom - 2,
    })
}

/// 可能连在某个控制台上的前台程序。
const CONSOLE_CLIENTS: [&str; 20] = [
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "wsl.exe",
    "wslhost.exe",
    "bash.exe",
    "ssh.exe",
    "python.exe",
    "pythonw.exe",
    "py.exe",
    "node.exe",
    "git.exe",
    "git-bash.exe",
    "mintty.exe",
    "far.exe",
    "conemuc64.exe",
    "conhost.exe",
    "vim.exe",
    "nvim.exe",
    "nano.exe",
];

unsafe fn console_candidates() -> Vec<u32> {
    let mut pids = Vec::new();
    let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
        return pids;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    if Process32FirstW(snapshot, &mut entry).is_ok() {
        loop {
            let name = exe_name(&entry);
            if CONSOLE_CLIENTS.iter().any(|known| *known == name) {
                pids.push(entry.th32ProcessID);
            }
            if Process32NextW(snapshot, &mut entry).is_err() {
                break;
            }
        }
    }
    let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    pids
}

fn exe_name(entry: &PROCESSENTRY32W) -> String {
    let end = entry
        .szExeFile
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..end]).to_ascii_lowercase()
}

unsafe fn window_class(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = GetClassNameW(window, &mut buffer);
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_stops_at_nul() {
        let mut entry = PROCESSENTRY32W::default();
        for (index, unit) in "cmd.exe".encode_utf16().enumerate() {
            entry.szExeFile[index] = unit;
        }
        assert_eq!(exe_name(&entry), "cmd.exe");
    }

    #[test]
    fn exe_name_is_lowercased() {
        let mut entry = PROCESSENTRY32W::default();
        for (index, unit) in "PowerShell.EXE".encode_utf16().enumerate() {
            entry.szExeFile[index] = unit;
        }
        assert_eq!(exe_name(&entry), "powershell.exe");
    }
}
