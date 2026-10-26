//! Text injection: send arbitrary Unicode to the focused window.
//!
//! `KEYEVENTF_UNICODE` bypasses the keyboard layout entirely, so committing
//! Japanese, German umlauts or emoji works the same in Notepad, Chrome,
//! WeChat and Word. Every injected event carries `MAGIC` in `dwExtraInfo` so
//! our own low-level hook recognises it and passes it straight through
//! (otherwise the IME would re-eat its own output).

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE,
};

/// Marker stored in `dwExtraInfo` for events produced by this process.
/// (32 位构建下 `usize` 只有 4 字节，所以按目标位宽取一个稳定的常量。)
#[cfg(target_pointer_width = "64")]
pub const MAGIC: usize = 0x4D43_5445_494E_5055; // "MCTEINPU"
#[cfg(target_pointer_width = "32")]
pub const MAGIC: usize = 0x4D43_5445; // "MCTE"

const CHUNK: usize = 64;

pub fn type_text(text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    for chunk in units.chunks(CHUNK) {
        send_units(chunk, false);
        send_units(chunk, true);
    }
}

/// Send `Chr`/backspace-style single keys is not needed for v1: every commit is
/// text, and the hook already swallowed whatever the user typed.
fn send_units(units: &[u16], up: bool) {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(units.len());
    for unit in units {
        let flags = if up {
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
        } else {
            KEYEVENTF_UNICODE
        };
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: Default::default(),
                    wScan: *unit,
                    dwFlags: KEYBD_EVENT_FLAGS(flags.0),
                    time: 0,
                    dwExtraInfo: MAGIC,
                },
            },
        });
    }
    if inputs.is_empty() {
        return;
    }
    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent == 0 {
            crate::logging::log(&format!(
                "inject: SendInput failed with error {}",
                std::io::Error::last_os_error()
            ));
        }
    }
}
