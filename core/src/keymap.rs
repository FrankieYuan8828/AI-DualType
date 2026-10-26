//! Virtual-key -> engine key mapping, shared by the hook backend and the TSF
//! text service so both interpret a keystroke identically.

use crate::engine::Key;

/// Map a Win32 virtual-key code to an IME key. `None` means "not ours".
pub fn map_vk(vk: u32) -> Option<Key> {
    match vk {
        // A..Z
        0x41..=0x5A => Some(Key::Char(char::from(b'a' + (vk as u8 - 0x41)))),
        // 0..9
        0x30..=0x39 => Some(Key::Char(char::from(b'0' + (vk as u8 - 0x30)))),
        0x20 => Some(Key::Space),
        0x0D => Some(Key::Enter),
        0x08 => Some(Key::Backspace),
        0x1B => Some(Key::Escape),
        0x25 => Some(Key::Left),
        0x26 => Some(Key::Up),
        0x27 => Some(Key::Right),
        0x28 => Some(Key::Down),
        // 标点（不按 Shift）：直接当字符交给引擎，由引擎转中文标点
        0xBE => Some(Key::Char('.')),
        0xBC => Some(Key::Char(',')),
        0xBA => Some(Key::Char(';')),
        0xDE => Some(Key::Char('\'')),
        0xBD => Some(Key::Char('-')),
        0xBB => Some(Key::Char('=')),
        0xDB => Some(Key::Char('[')),
        0xDD => Some(Key::Char(']')),
        0xDC => Some(Key::Char('\\')),
        0xBF => Some(Key::Char('/')),
        0xC0 => Some(Key::Char('`')),
        // 翻页：PageUp / PageDown（`+` 是 Shift+'='，见 shifted_punctuation）
        0x21 => Some(Key::PagePrev),
        0x22 => Some(Key::PageNext),

        _ => None,
    }
}

/// Shift + 标点/数字 对应的 ASCII 字符（`!` `(` `”` 这些只有按住 Shift 才出得来）。
pub fn shifted_punctuation(vk: u32) -> Option<char> {
    Some(match vk {
        0x31 => '!',  // !
        0x32 => '@',  // @
        0x33 => '#',  // #
        0x34 => '$',  // $
        0x35 => '%',  // %
        0x36 => '^',  // ^（……）
        0x37 => '&',  // &
        0x38 => '*',  // *
        0x39 => '(',  // （
        0x30 => ')',  // ）
        0xBD => '_',  // ——
        0xBB => '+',  // ＋
        0xDB => '{',  // 『
        0xDD => '}',  // 』
        0xDC => '|',  // ｜
        0xBF => '?',  // ？
        0xBE => '>',  // 》
        0xBC => '<',  // 《
        0xBA => ':',  // ：
        0xDE => '"',  // “ ”
        0xC0 => '~',  // ～
        _ => return None,
    })
}

/// 翻页键即使在按着 Shift 时也要交给我们（`+` 就是 Shift+'='）。
pub fn is_paging_key(vk: u32) -> bool {
    matches!(vk, 0x21 | 0x22)
}

/// Keys the IME can never swallow: modifiers, function keys, etc.
pub fn is_modifier(vk: u32) -> bool {
    matches!(
        vk,
        0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0x14 | 0x90 | 0x91 | 0xA0..=0xA5 | 0x5D
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_letters_to_lowercase() {
        assert_eq!(map_vk(0x41), Some(Key::Char('a')));
        assert_eq!(map_vk(0x5A), Some(Key::Char('z')));
    }

    #[test]
    fn maps_editing_keys() {
        assert_eq!(map_vk(0x0D), Some(Key::Enter));
        assert_eq!(map_vk(0x20), Some(Key::Space));
        assert_eq!(map_vk(0x08), Some(Key::Backspace));
        assert_eq!(map_vk(0xBE), Some(Key::Char('.')));
    }

    #[test]
    fn unknown_keys_are_not_ours() {
        assert_eq!(map_vk(0x70), None); // F1
        assert!(is_modifier(0x11));
        assert!(!is_modifier(0x41));
    }
}
