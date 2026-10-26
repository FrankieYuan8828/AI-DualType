//! 语言栏按钮（任务栏右下角那个「输入指示器」图标）。
//!
//! 为什么需要它：Win+Space 列表里的图标来自 `RegisterProfile` 的 `IconFile`（我们已经对了），
//! 但**任务栏输入指示器**用的是该 TIP 注册的语言栏按钮图标。没实现按钮时，
//! Windows 只能画一个通用占位图标（用户截图里红圈那个带蓝点的小圈箭头）——
//! 这正是「图标错误」的根因。Weasel 也是这样做的（它的 `WeaselTSF/LanguageBar.cpp`）。
//!
//! 顺带把左键点击做成中/英切换，和微软拼音的交互一致。

use windows::core::{implement, BOOL, BSTR, IUnknownImpl, Result};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::System::Com::CoTaskMemAlloc;
use windows::core::GUID;
use windows::Win32::UI::TextServices::{
    ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItem_Impl, ITfMenu,
    TfLBIClick, TF_LANGBARITEMINFO, TF_LBI_CLK_LEFT, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_BTN_MENU,
    TF_LBI_STYLE_SHOWNINTRAY,
};

/// 自己的语言栏按钮 GUID。
///
/// 注意：不能用 `GUID_LBI_INPUTMODE`——那是系统保留项（输入模式指示器），
/// `ITfLangBarItemMgr::AddItem` 对它返回 E_UNEXPECTED(0x80004005)。
/// Weasel 也是注册自己的 GUID（`c_guidLangBarItemButton`）。
const GUID_LANGBAR_BUTTON: GUID = GUID::from_u128(0x341f_9e3a_b8ad_499d_936c_4870_1e32_9fb2);
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, LoadImageW, HICON, IMAGE_ICON, LR_LOADFROMFILE,
};

use crate::{CLSID_TEXTSERVICE, DISPLAY_NAME};

/// 把 Rust 字符串放到 CoTaskMemAlloc 的缓冲区里 —— TSF 会用 CoTaskMemFree 释放它。
/// （不能用 `BSTR::from`：那是 SysAllocString，前面还有 4 字节长度前缀，用错释放器会毁堆。）
fn co_task_string(text: &str) -> BSTR {
    let units: Vec<u16> = text.encode_utf16().chain([0]).collect();
    unsafe {
        let raw = CoTaskMemAlloc(units.len() * std::mem::size_of::<u16>()) as *mut u16;
        if raw.is_null() {
            return BSTR::new();
        }
        std::ptr::copy_nonoverlapping(units.as_ptr(), raw, units.len());
        BSTR::from_raw(raw)
    }
}

/// 语言栏按钮：图标直接从 `mcte.ico` 读，避免依赖 DLL 资源。
#[implement(ITfLangBarItemButton, ITfLangBarItem)]
pub struct LangBarButton {
    pub icon: HICON,
    pub tooltip: String,
    toggle: std::sync::Mutex<Box<dyn FnMut()>>,
}

impl LangBarButton {
    pub fn new(icon_path: Option<&std::path::Path>, tooltip: String, toggle: impl FnMut() + 'static) -> Self {
        let icon = load_icon(icon_path).unwrap_or_default();
        Self {
            icon,
            tooltip,
            toggle: std::sync::Mutex::new(Box::new(toggle)),
        }
    }
}

fn load_icon(path: Option<&std::path::Path>) -> Option<HICON> {
    let path = path?;
    let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain([0]).collect();
    unsafe {
        LoadImageW(
            None,
            windows::core::PCWSTR(wide.as_ptr()),
            IMAGE_ICON,
            32,
            32,
            LR_LOADFROMFILE,
        )
        .ok()
        .map(|handle| HICON(handle.0))
    }
}

impl Drop for LangBarButton {
    fn drop(&mut self) {
        if !self.icon.is_invalid() {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(self.icon);
            }
        }
    }
}

impl ITfLangBarItem_Impl for LangBarButton_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> Result<()> {
        if pinfo.is_null() {
            return Err(windows::Win32::Foundation::E_POINTER.into());
        }
        let inner = self.get_impl();
        unsafe {
            let info = &mut *pinfo;
            *info = TF_LANGBARITEMINFO {
                clsidService: CLSID_TEXTSERVICE,
                guidItem: GUID_LANGBAR_BUTTON,
                // 与 Weasel 一致：按钮 + 菜单 + 「显示在通知区域」
                dwStyle: TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_BTN_MENU | TF_LBI_STYLE_SHOWNINTRAY,
                ulSort: 1,
                szDescription: [0; 32],
            };
            let name: Vec<u16> = DISPLAY_NAME.encode_utf16().collect();
            for (slot, unit) in info.szDescription.iter_mut().zip(name) {
                *slot = unit;
            }
        }
        let _ = inner;
        Ok(())
    }

    fn GetStatus(&self) -> Result<u32> {
        Ok(0) // 0 = 显示且可用
    }

    fn Show(&self, _fshow: BOOL) -> Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        Ok(co_task_string(&self.get_impl().tooltip))
    }
}

impl ITfLangBarItemButton_Impl for LangBarButton_Impl {
    fn OnClick(&self, click: TfLBIClick, _pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        if click == TF_LBI_CLK_LEFT {
            if let Ok(mut guard) = self.get_impl().toggle.lock() {
                (guard)();
            }
        }
        Ok(())
    }

    fn InitMenu(&self, _pmenu: windows::core::Ref<'_, ITfMenu>) -> Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _wid: u32) -> Result<()> {
        Ok(())
    }

    fn GetIcon(&self) -> Result<HICON> {
        let icon = self.get_impl().icon;
        if icon.is_invalid() {
            return Err(windows::Win32::Foundation::E_FAIL.into());
        }
        // 调用方会接管并负责销毁，所以给一份拷贝。
        unsafe { CopyIcon(icon) }
    }

    fn GetText(&self) -> Result<BSTR> {
        // 输入指示器只显示图标，不需要文字
        Ok(BSTR::new())
    }
}

