//! MC-TypeEnglish 的 TSF 文本服务（`mcte_tsf.dll`）。
//!
//! 它和钩子版（`mcte.exe`）共用 `mcte_core`：同一套拼音引擎、词库、自学习用户库与
//! 候选窗绘制。区别只在「按键从哪来」和「文字怎么进文档」：
//!
//! ```text
//!   钩子版： WH_KEYBOARD_LL              -> engine -> SendInput(unicode)
//!   TSF 版： ITfKeyEventSink::OnKeyDown  -> engine -> ITfInsertAtSelection::InsertTextAtSelection
//! ```
//!
//! 因为 TSF 服务是**加载进宿主进程**的 in-proc COM 组件，它天然能输入到管理员权限的
//! 窗口里（这是钩子版做不到的），并且注册后会出现在 Win+Space 的输入法列表里。
//!
//! 注册方式（需要管理员）：
//! ```text
//!   regsvr32 /s "C:\Program Files\MC-TypeEnglish\mcte_tsf.dll"      (64 位宿主)
//!   C:\Windows\SysWOW64\regsvr32 /s "...\mcte_tsf_x86.dll"           (32 位宿主)
//! ```
//! 或直接跑安装器 `Install.ps1`。

#![cfg(windows)]

mod langbar;
mod registry;
mod service;

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

use windows::core::{implement, GUID, IUnknown, Interface, Result as WinResult, HRESULT};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_INVALIDARG, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows::Win32::Foundation::HMODULE;

/// 文本服务的 CLSID（注册表里 CLSID\...\InprocServer32 就是这个键）。
pub const CLSID_TEXTSERVICE: GUID = GUID::from_u128(0x7c3e_0c11_5f6a_4e52_9b2c_a1d4_7b2e9f31);
/// 语言配置（profile）的 GUID，决定 Win+Space 列表里的那一项。
pub const GUID_PROFILE: GUID = GUID::from_u128(0xb3d2_f0a4_6c81_4e17_9e45_2d8a_9c6b4f72);
/// 简体中文（0x0804）
pub const LANGID_ZH_CN: u16 = 0x0804;
/// 输入法显示名
pub const DISPLAY_NAME: &str = "MC-TypeEnglish";

/// DLL 引用计数：-1 表示没有活对象（与 ATL 的 `g_cRefDll` 同语义）。
static DLL_REF: AtomicI32 = AtomicI32::new(-1);

pub fn dll_add_ref() {
    DLL_REF.fetch_add(1, Ordering::SeqCst);
}

pub fn dll_release() {
    DLL_REF.fetch_sub(1, Ordering::SeqCst);
}

/// 本 DLL 的完整路径（注册 InprocServer32 与 profile 图标时用）。
pub fn dll_path() -> String {
    unsafe {
        let mut module = HMODULE::default();
        // 用本 DLL 里一个具体函数的地址反查 HMODULE（避免依赖 DllMain 保存的 hInstance）。
        let addr = DllCanUnloadNow as *const c_void as *const u16;
        let _ = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            windows::core::PCWSTR(addr),
            &mut module,
        );
        let mut buffer = [0u16; 1024];
        let len = GetModuleFileNameW(Some(module), &mut buffer);
        String::from_utf16_lossy(&buffer[..len as usize])
    }
}

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> WinResult<()> {
        unsafe {
            if !punkouter.is_null() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            if riid.is_null() || ppvobject.is_null() {
                return Err(E_INVALIDARG.into());
            }
            let requested = *riid;
            let unknown: IUnknown = service::TextService::new().into();

            macro_rules! provide {
                ($iface:ty) => {
                    if requested == <$iface as Interface>::IID {
                        let iface: $iface = unknown.cast()?;
                        *ppvobject = iface.into_raw();
                        return Ok(());
                    }
                };
            }
            provide!(IUnknown);
            provide!(windows::Win32::UI::TextServices::ITfTextInputProcessor);
            provide!(windows::Win32::UI::TextServices::ITfTextInputProcessorEx);
            provide!(windows::Win32::UI::TextServices::ITfKeyEventSink);
            Err(CLASS_E_CLASSNOTAVAILABLE.into())
        }
    }

    fn LockServer(&self, flock: windows::core::BOOL) -> WinResult<()> {
        if flock.as_bool() {
            dll_add_ref();
        } else {
            dll_release();
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return E_INVALIDARG;
        }
        if *rclsid != CLSID_TEXTSERVICE {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IClassFactory = ClassFactory.into();
        let requested = *riid;
        if requested == IClassFactory::IID {
            *ppv = factory.into_raw();
            return S_OK;
        }
        if requested == IUnknown::IID {
            match factory.cast::<IUnknown>() {
                Ok(unknown) => {
                    *ppv = unknown.into_raw();
                    S_OK
                }
                Err(err) => err.code(),
            }
        } else {
            CLASS_E_CLASSNOTAVAILABLE
        }
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF.load(Ordering::SeqCst) >= 0 {
        S_FALSE
    } else {
        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register() {
        Ok(()) => S_OK,
        Err(err) => {
            mcte_core::logging::log(&format!("DllRegisterServer failed: {err}"));
            windows::Win32::Foundation::E_FAIL
        }
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    match registry::unregister() {
        Ok(()) => S_OK,
        Err(err) => {
            mcte_core::logging::log(&format!("DllUnregisterServer failed: {err}"));
            windows::Win32::Foundation::E_FAIL
        }
    }
}
