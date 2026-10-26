//! 注册 / 反注册（`regsvr32 mcte_tsf.dll` 走的就是这里）。
//!
//! 三件事，缺一不可：
//!
//! 1. `HKCR\CLSID\{CLSID}\InprocServer32` → 指向 DLL，`ThreadingModel = Apartment`；
//!    （32 位宿主读的是 `WOW6432Node` 下的同一分支，所以 32 位 DLL 也要单独 regsvr32 一次）
//! 2. TSF category → 让 Windows 知道「这是一个键盘类输入法」；
//! 3. `ITfInputProcessorProfileMgr::RegisterProfile` → 在 Win+Space 语言列表里出现。
//!
//! 注意 Weasel 上游的两个坑（本实现没有照抄）：
//! * 它的 `UnregisterServer` 里 TIP 键前缀拼成 `Microsft`，卸载其实删不掉；
//! * `RegisterProfile` 的 `cchDesc/cchFile` 它传的是字节数，中文描述会被截断。

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    ITfCategoryMgr, ITfInputProcessorProfileMgr, ITfInputProcessorProfiles, CLSID_TF_CategoryMgr,
    CLSID_TF_InputProcessorProfiles, GUID_TFCAT_CATEGORY_OF_TIP, GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
};

use crate::{CLSID_TEXTSERVICE, DISPLAY_NAME, GUID_PROFILE, LANGID_ZH_CN};

/// 键盘类输入法必须注册的 category；其余几个是「安全模式 / 输入模式隔离仓 /
/// UI 元素」的能力声明，成本为零，一起登记更规范。
#[cfg_attr(target_pointer_width = "32", allow(dead_code))]
const CATEGORIES: [GUID; 5] = [
    GUID_TFCAT_CATEGORY_OF_TIP,
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
];

pub fn register() -> Result<(), String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let dll = crate::dll_path();
    if dll.is_empty() {
        return Err("cannot resolve own DLL path".to_string());
    }
    write_clsid(&dll)?;

    // 32 位 DLL 的 DllRegisterServer 只写 CLSID（会落到 WOW6432Node），
    // profile / category 是机器级、与位数无关的，只在 64 位这一侧注册一次。
    #[cfg(target_pointer_width = "64")]
    {
        register_categories()?;
        register_profile(&dll)?;
        match enable_profile_for_user() {
            Ok(()) => mcte_core::logging::log("tsf: enabled language profile for current user"),
            Err(err) => mcte_core::logging::log(&format!("tsf: EnableLanguageProfile failed: {err}")),
        }
    }
    Ok(())
}

pub fn unregister() -> Result<(), String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    #[cfg(target_pointer_width = "64")]
    {
        let _ = unregister_profile();
        let _ = unregister_categories();
    }
    let clsid = clsid_key_name();
    let wide: Vec<u16> = clsid.encode_utf16().chain([0]).collect();
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(wide.as_ptr()));
    }
    Ok(())
}

fn clsid_key_name() -> String {
    format!("CLSID\\{}", guid_string(&CLSID_TEXTSERVICE))
}

fn guid_string(guid: &GUID) -> String {
    let bytes = guid.to_u128();
    let b = bytes.to_be_bytes();
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn check(err: WIN32_ERROR, what: &str) -> Result<(), String> {
    if err == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("{what} failed: {}", err.0))
    }
}

fn create_key(root: HKEY, path: &str) -> Result<HKEY, String> {
    let path = wide(path);
    let mut key = HKEY::default();
    unsafe {
        check(
            RegCreateKeyExW(
                root,
                PCWSTR(path.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut key,
                None,
            ),
            "RegCreateKeyExW",
        )?;
    }
    Ok(key)
}

fn set_string(key: HKEY, name: Option<&str>, value: &str) -> Result<(), String> {
    let name_wide = name.map(wide);
    let value_wide = wide(value);
    let bytes = unsafe {
        std::slice::from_raw_parts(
            value_wide.as_ptr() as *const u8,
            value_wide.len() * std::mem::size_of::<u16>(),
        )
    };
    let name_ptr = match &name_wide {
        Some(text) => PCWSTR(text.as_ptr()),
        None => PCWSTR::null(),
    };
    unsafe {
        check(
            RegSetValueExW(key, name_ptr, None, REG_SZ, Some(bytes)),
            "RegSetValueExW",
        )
    }
}

fn write_clsid(dll_path: &str) -> Result<(), String> {
    let clsid = create_key(HKEY_CLASSES_ROOT, &clsid_key_name())?;
    let result = (|| {
        set_string(clsid, None, DISPLAY_NAME)?;
        let inproc = create_key(clsid, "InprocServer32")?;
        let inner = (|| {
            set_string(inproc, None, dll_path)?;
            set_string(inproc, Some("ThreadingModel"), "Apartment")
        })();
        unsafe {
            let _ = RegCloseKey(inproc);
        }
        inner
    })();
    unsafe {
        let _ = RegCloseKey(clsid);
    }
    result
}

#[allow(dead_code)]
fn register_categories() -> Result<(), String> {
    let mgr: ITfCategoryMgr = unsafe {
        CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(CLSID_TF_CategoryMgr): {e}"))?
    };
    for category in CATEGORIES {
        unsafe {
            mgr.RegisterCategory(&CLSID_TEXTSERVICE, &category, &CLSID_TEXTSERVICE)
                .map_err(|e| format!("RegisterCategory: {e}"))?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn unregister_categories() -> Result<(), String> {
    let mgr: ITfCategoryMgr = unsafe {
        CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(CLSID_TF_CategoryMgr): {e}"))?
    };
    for category in CATEGORIES {
        unsafe {
            let _ = mgr.UnregisterCategory(&CLSID_TEXTSERVICE, &category, &CLSID_TEXTSERVICE);
        }
    }
    Ok(())
}

/// 「让这个 IME 出现在当前用户的 Win+Space 列表里」。
///
/// 关键区别（这是本项目的真实踩坑）：`RegisterProfile` 只写**机器级**的
/// `HKLM\SOFTWARE\Microsoft\CTF\TIP`，而 Win+Space 轮换的是**用户级**列表
/// `HKCU\Control Panel\International\User Profile\<lang>`。只写 HKLM 时，
/// 设置里看不到、Win+Space 也切不到，表现为「装了但用不了」。
#[allow(dead_code)]
fn enable_profile_for_user() -> Result<(), String> {
    let profiles: ITfInputProcessorProfiles = unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(ITfInputProcessorProfiles): {e}"))?
    };
    unsafe {
        profiles
            .EnableLanguageProfile(&CLSID_TEXTSERVICE, LANGID_ZH_CN, &GUID_PROFILE, true)
            .map_err(|e| format!("EnableLanguageProfile: {e}"))?;
        // 新用户/新语言首次建列表时也默认带上（失败不影响已登录用户）。
        let _ = profiles.EnableLanguageProfileByDefault(
            &CLSID_TEXTSERVICE,
            LANGID_ZH_CN,
            &GUID_PROFILE,
            true,
        );
    }
    Ok(())
}

#[allow(dead_code)]
/// 图标文件：优先用 DLL 同级或上一级的 `mcte.ico`，没有再退回 DLL 自身。
fn icon_path_for(dll_path: &str) -> String {
    let dll = std::path::Path::new(dll_path);
    let mut candidates = Vec::new();
    if let Some(dir) = dll.parent() {
        candidates.push(dir.join("mcte.ico"));
        if let Some(parent) = dir.parent() {
            candidates.push(parent.join("mcte.ico"));
        }
    }
    for candidate in candidates {
        if candidate.is_file() {
            return candidate.to_string_lossy().to_string();
        }
    }
    dll_path.to_string()
}

fn register_profile(dll_path: &str) -> Result<(), String> {
    let mgr: ITfInputProcessorProfileMgr = unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(CLSID_TF_InputProcessorProfiles): {e}"))?
    };
    let desc = wide(DISPLAY_NAME);
    // 语言栏/输入法列表里显示 MC 图标（而不是系统给的默认「简」）
    let icon = wide(&icon_path_for(dll_path));
    unsafe {
        mgr.RegisterProfile(
            &CLSID_TEXTSERVICE,
            LANGID_ZH_CN,
            &GUID_PROFILE,
            &desc,
            &icon,
            0,
            HKL::default(),
            0,
            true,
            0,
        )
        .map_err(|e| format!("RegisterProfile: {e}"))?;
    }
    Ok(())
}

#[allow(dead_code)]
fn unregister_profile() -> Result<(), String> {
    let mgr: ITfInputProcessorProfileMgr = unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(CLSID_TF_InputProcessorProfiles): {e}"))?
    };
    unsafe {
        mgr.UnregisterProfile(&CLSID_TEXTSERVICE, LANGID_ZH_CN, &GUID_PROFILE, 0)
            .map_err(|e| format!("UnregisterProfile: {e}"))?;
    }
    Ok(())
}

/// 给安装器/自检用：把当前注册状态读回来（不写注册表）。
#[allow(dead_code)]
pub fn registration_report() -> String {
    let clsid = guid_string(&CLSID_TEXTSERVICE);
    let profile = guid_string(&GUID_PROFILE);
    format!(
        "clsid={clsid} profile={profile} langid=0x{LANGID_ZH_CN:04X} dll={}",
        crate::dll_path()
    )
}


