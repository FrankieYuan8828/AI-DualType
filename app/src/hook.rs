//! Global low-level keyboard hook.
//!
//! `WH_KEYBOARD_LL` is installed on the UI thread, so callbacks arrive in that
//! thread's message loop — that is what lets the whole IME live in a
//! single-threaded state machine with no locks.
//!
//! Known limit of this backend (documented in the README): Windows does not
//! deliver hook events for windows running at a higher integrity level than
//! this process, so an unelevated install cannot type into an elevated app.

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD_LL,
};

pub type HookProc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

pub struct KeyboardHook {
    hook: HHOOK,
}

impl KeyboardHook {
    pub fn install(proc: HookProc) -> Result<Self, String> {
        unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0)
                .map_err(|e| format!("SetWindowsHookExW failed: {e}"))?;
            if hook.is_invalid() {
                return Err("SetWindowsHookExW returned an invalid hook".to_string());
            }
            Ok(Self { hook })
        }
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.hook);
        }
    }
}

/// Human-readable backend description, printed by `--probe` so the verification
/// output states which mechanism is active.
pub fn describe_backend() -> &'static str {
    "WH_KEYBOARD_LL (global low-level keyboard hook)"
}

/// Helper for the callback to forward a key to the next hook / the application.
pub fn pass_through(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
