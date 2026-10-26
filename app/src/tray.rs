//! Tray icon and menu.
//!
//! Deliberately minimal (the TypeAnything README calls out 4 noisy tray items
//! as a UX bug): toggle mode, reload, open the config/data folders, quit.

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetForegroundWindow, LoadIconW,
    LoadImageW, PostMessageW, SetForegroundWindow, TrackPopupMenu, HICON, IMAGE_ICON, LR_LOADFROMFILE,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, TPM_RETURNCMD, WM_NULL,
};

/// 优先加载安装目录里的 `mcte.ico`（MC 图标），失败再退回系统图标。
fn load_app_icon() -> HICON {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("mcte.ico"));
            candidates.push(dir.join("data").join("mcte.ico"));
            candidates.push(dir.join("..").join("mcte.ico"));
            candidates.push(dir.join("..").join("data").join("mcte.ico"));
        }
    }
    unsafe {
        let cx = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON,
        );
        let cy = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CYSMICON,
        );
        for candidate in candidates {
            if !candidate.exists() {
                continue;
            }
            let wide: Vec<u16> = candidate.to_string_lossy().encode_utf16().chain([0]).collect();
            let handle = LoadImageW(
                None,
                windows::core::PCWSTR(wide.as_ptr()),
                IMAGE_ICON,
                cx,
                cy,
                LR_LOADFROMFILE,
            );
            if let Ok(handle) = handle {
                if !handle.is_invalid() {
                    return HICON(handle.0);
                }
            }
        }
        LoadIconW(None, w!("IDI_APPLICATION")).unwrap_or_default()
    }
}

pub const ID_TOGGLE_MODE: usize = 1001;
pub const ID_RELOAD: usize = 1002;
pub const ID_OPEN_CONFIG_DIR: usize = 1003;
pub const ID_OPEN_DATA_DIR: usize = 1004;
pub const ID_QUIT: usize = 1005;
const ID_STATUS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ToggleMode,
    Reload,
    OpenConfigDir,
    OpenDataDir,
    Quit,
    None,
}

pub struct Tray {
    hwnd: HWND,
    icon_added: bool,
}

impl Tray {
    pub fn add(hwnd: HWND, callback_message: u32, tooltip: &str) -> Result<Self, String> {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: callback_message,
            hIcon: load_app_icon(),
            ..Default::default()
        };
        write_tip(&mut data, tooltip);
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
        if !ok.as_bool() {
            return Err("Shell_NotifyIconW(NIM_ADD) failed".to_string());
        }
        Ok(Self {
            hwnd,
            icon_added: true,
        })
    }

    pub fn set_tooltip(&self, tooltip: &str) {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_TIP,
            ..Default::default()
        };
        write_tip(&mut data, tooltip);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    /// Show the context menu and return the chosen action.
    pub fn popup(&self, status_line: &str, mode_line: &str) -> TrayAction {
        unsafe {
            let Ok(menu) = CreatePopupMenu() else {
                return TrayAction::None;
            };
            let status: Vec<u16> = status_line.encode_utf16().chain([0]).collect();
            let mode: Vec<u16> = mode_line.encode_utf16().chain([0]).collect();
            let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, ID_STATUS, windows::core::PCWSTR(status.as_ptr()));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
            let _ = AppendMenuW(menu, MF_STRING, ID_TOGGLE_MODE, windows::core::PCWSTR(mode.as_ptr()));
            let _ = AppendMenuW(menu, MF_STRING, ID_RELOAD, w!("重新加载配置与词库"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
            let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_CONFIG_DIR, w!("打开配置文件夹"));
            let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_DATA_DIR, w!("打开词库文件夹"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
            let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("退出 MC-TypeEnglish"));

            let mut point = POINT::default();
            let _ = GetCursorPos(&mut point);
            let _ = SetForegroundWindow(self.hwnd);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD,
                point.x,
                point.y,
                Some(0),
                self.hwnd,
                None,
            );
            // Required so the menu closes when the user clicks elsewhere.
            let _ = PostMessageW(Some(GetForegroundWindow()), WM_NULL, Default::default(), Default::default());
            let _ = DestroyMenu(menu);

            match command.0 as usize {
                ID_TOGGLE_MODE => TrayAction::ToggleMode,
                ID_RELOAD => TrayAction::Reload,
                ID_OPEN_CONFIG_DIR => TrayAction::OpenConfigDir,
                ID_OPEN_DATA_DIR => TrayAction::OpenDataDir,
                ID_QUIT => TrayAction::Quit,
                _ => TrayAction::None,
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        if self.icon_added {
            let data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                ..Default::default()
            };
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
        }
    }
}

fn write_tip(data: &mut NOTIFYICONDATAW, tooltip: &str) {
    let units: Vec<u16> = tooltip.encode_utf16().take(data.szTip.len() - 1).collect();
    for (slot, unit) in data.szTip.iter_mut().zip(units) {
        *slot = unit;
    }
}

#[allow(dead_code)]
fn _unused(lparam: LPARAM) -> isize {
    lparam.0
}
