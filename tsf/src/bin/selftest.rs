//! `mcte-tsf-selftest`：不需要管理员权限就能验证 TSF DLL 的 COM 面。
//!
//! 做的事（全部通过真实的 COM 调用，不是读源码猜）：
//!   1. LoadLibrary 加载 DLL，`GetProcAddress` 拿到 4 个导出函数；
//!   2. `DllCanUnloadNow` 在无对象时应返回 S_OK；
//!   3. `DllGetClassObject(CLSID, IID_IClassFactory)` 拿到类工厂；
//!   4. `IClassFactory::CreateInstance(..., IID_ITfTextInputProcessorEx)` 真的创建出文本服务；
//!   5. 创建后 `DllCanUnloadNow` 必须变成 S_FALSE（说明引用计数生效）；
//!   6. 引用全部释放后回到 S_OK（说明 Drop 里的计数配对正确），并 FreeLibrary 卸载。
//!
//! 用法：`cargo run -p mcte-tsf --bin mcte-tsf-selftest -- <path-to-mcte_tsf.dll>`

use std::ffi::c_void;
use std::ptr;

use windows::core::{s, GUID, Interface, HRESULT};
use windows::Win32::Foundation::{S_FALSE, S_OK};
use windows::Win32::System::Com::{CoInitializeEx, IClassFactory, COINIT_APARTMENTTHREADED};
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::TextServices::ITfTextInputProcessorEx;

/// 必须与 `tsf/src/lib.rs` 里的 CLSID_TEXTSERVICE 一致。
const CLSID_TEXTSERVICE: GUID = GUID::from_u128(0x7c3e_0c11_5f6a_4e52_9b2c_a1d4_7b2e9f31);

type DllGetClassObjectFn =
    unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;
type DllCanUnloadNowFn = unsafe extern "system" fn() -> HRESULT;

fn main() -> Result<(), String> {
    let dll = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/release/mcte_tsf.dll".to_string());
    let path = std::path::Path::new(&dll);
    if !path.is_file() {
        return Err(format!("找不到 DLL: {dll}（先用 cargo build -p mcte-tsf --release 构建）"));
    }

    let mut failures = 0usize;
    let mut check = |name: &str, ok: bool, detail: String| {
        if ok {
            println!("PASS  {name}  {detail}");
        } else {
            failures += 1;
            println!("FAIL  {name}  {detail}");
        }
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let wide: Vec<u16> = dll.encode_utf16().chain([0]).collect();
        let module = LoadLibraryW(windows::core::PCWSTR(wide.as_ptr()))
            .map_err(|e| format!("LoadLibraryW({dll}) 失败: {e}"))?;

        let get_class = GetProcAddress(module, s!("DllGetClassObject"))
            .ok_or("缺少导出 DllGetClassObject")?;
        let can_unload = GetProcAddress(module, s!("DllCanUnloadNow"))
            .ok_or("缺少导出 DllCanUnloadNow")?;
        let get_class: DllGetClassObjectFn = std::mem::transmute(get_class);
        let can_unload: DllCanUnloadNowFn = std::mem::transmute(can_unload);

        check(
            "导出函数齐全",
            GetProcAddress(module, s!("DllRegisterServer")).is_some()
                && GetProcAddress(module, s!("DllUnregisterServer")).is_some(),
            "DllGetClassObject / DllCanUnloadNow / DllRegisterServer / DllUnregisterServer".to_string(),
        );

        let before = can_unload();
        check(
            "无对象时可以卸载",
            before == S_OK,
            format!("DllCanUnloadNow = 0x{:08X} (S_OK=0)", before.0),
        );

        let mut factory_ptr: *mut c_void = ptr::null_mut();
        let hr = get_class(&CLSID_TEXTSERVICE, &IClassFactory::IID, &mut factory_ptr);
        check(
            "DllGetClassObject 返回类工厂",
            hr == S_OK && !factory_ptr.is_null(),
            format!("hr = 0x{:08X}", hr.0),
        );
        if factory_ptr.is_null() {
            let _ = FreeLibrary(module);
            return Err("类工厂创建失败，后续检查跳过".to_string());
        }
        let factory: IClassFactory = IClassFactory::from_raw(factory_ptr);

        let created: Result<ITfTextInputProcessorEx, _> = factory.CreateInstance(None);
        let created_ok = created.is_ok();
        check(
            "CreateInstance 创建 ITfTextInputProcessorEx",
            created_ok,
            match &created {
                Ok(_) => "hr = 0x00000000".to_string(),
                Err(err) => format!("hr = 0x{:08X}", err.code().0),
            },
        );

        if let Ok(object) = created {
            let during = can_unload();
            check(
                "有活对象时拒绝卸载",
                during == S_FALSE,
                format!("DllCanUnloadNow = 0x{:08X} (S_FALSE=1)", during.0),
            );
            let base = object.cast::<windows::Win32::UI::TextServices::ITfTextInputProcessor>();
            check(
                "ITfTextInputProcessorEx 可 QI 到 ITfTextInputProcessor",
                base.is_ok(),
                format!("{:?}", base.as_ref().err().map(|e| e.code().0)),
            );
            let not_supported = object.cast::<windows::Win32::UI::TextServices::ITfEditSession>();
            check(
                "未实现的接口被正确拒绝",
                not_supported.is_err(),
                "ITfEditSession 不在文本服务对象上".to_string(),
            );
            drop(object);
        }

        drop(factory);
        let after = can_unload();
        check(
            "释放后回到可卸载状态",
            after == S_OK,
            format!("DllCanUnloadNow = 0x{:08X} (S_OK=0)", after.0),
        );

        let _ = FreeLibrary(module);
    }

    println!();
    if failures > 0 {
        println!("SELFTEST FAILED ({failures})");
        Err(format!("{failures} checks failed"))
    } else {
        println!("SELFTEST PASSED");
        Ok(())
    }
}
