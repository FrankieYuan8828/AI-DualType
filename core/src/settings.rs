//! 跟随语言设置弹窗。
//!
//! 候选窗右上角的「设置」按钮打开它：里面列着「不额外语言」和几个可选语言，
//! 单选，点一下立刻生效并写回 `config.json`。英文永远跟随，这里只管第二种。
//!
//! 实现上和候选窗一样是 GDI 自绘的置顶小窗：不抢焦点（`WS_EX_NOACTIVATE`），
//! 点外部或失焦就自己收起，避免在桌面上留残影。

use std::cell::RefCell;

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, DeleteObject, EndPaint, FillRect, FrameRect,
    HGDIOBJ, HDC, InvalidateRect, SelectObject, SetBkMode, SetTextColor, TextOutW, CLEARTYPE_QUALITY,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, FONT_OUTPUT_PRECISION,
    LOGFONTW, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics, KillTimer,
    RegisterClassW, SetTimer, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST,
    MA_NOACTIVATE, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE,
    WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::config::EXTRA_LANGUAGES;

const CLASS_NAME_STR: &str = "MCTypeEnglishSettings";
const TITLE: &str = "跟随语言（英文固定）";
/// 「不额外语言」这一项排在最前。
const NONE_LABEL: &str = "不额外语言";

const PAD: i32 = 10;
const ROW_H: i32 = 30;
const TITLE_H: i32 = 26;
const WIDTH: i32 = 200;
const FONT_SIZE: i32 = 14;
const FOCUS_GUARD_TIMER: usize = 97;

const COLOR_BG: u32 = 0x00F7_F7F7;
const COLOR_BORDER: u32 = 0x00D6_D6D6;
const COLOR_TEXT: u32 = 0x0000_0000;
const COLOR_TITLE: u32 = 0x0070_7070;
const COLOR_HILITE: u32 = 0x00C0_6700;
const COLOR_HILITE_TEXT: u32 = 0x00FF_FFFF;
/// 鼠标滚轮移动到的行（还没点下去）。
const COLOR_HOVER: u32 = 0x00E8_F0FE;
const WM_MOUSEWHEEL: u32 = 0x020A;

struct State {
    /// 当前选中的语言代码（`None` = 不额外语言）。
    current: Option<String>,
    hover: Option<usize>,
    /// 列表滚动偏移（像素）。全部行放得下时恒为 0。
    scroll: i32,
    font: isize,
    guard_hwnd: isize,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static WINDOW_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static PICK_HANDLER: RefCell<Option<Box<dyn Fn(Option<String>)>>> = const { RefCell::new(None) };
}

/// 行序号 -> 语言代码：0 是「不额外语言」，其后按 `EXTRA_LANGUAGES` 顺序。
fn language_at(row: usize) -> Option<String> {
    if row == 0 {
        None
    } else {
        EXTRA_LANGUAGES.get(row - 1).map(|(code, _)| code.to_string())
    }
}

fn rows() -> usize {
    EXTRA_LANGUAGES.len() + 1
}

/// 列表区（标题下方）的高度。
fn list_height() -> i32 {
    rows() as i32 * ROW_H
}

/// 内容总高度（含标题与内边距）。
fn content_height() -> i32 {
    PAD * 2 + TITLE_H + list_height()
}

/// 窗口实际高度：内容放不下时最多占屏幕 60%，其余靠滚轮滚动。
fn window_height() -> i32 {
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    content_height().min(screen_h * 6 / 10)
}

/// 最多能滚动多少像素。
fn max_scroll() -> i32 {
    (content_height() - window_height()).max(0)
}

fn row_label(row: usize) -> &'static str {
    if row == 0 {
        NONE_LABEL
    } else {
        EXTRA_LANGUAGES
            .get(row - 1)
            .map(|(_, name)| *name)
            .unwrap_or("")
    }
}

/// 打开（或刷新）设置窗。`anchor` 是候选窗右上角的屏幕坐标。
pub fn open(anchor: POINT, current: Option<String>, on_pick: impl Fn(Option<String>) + 'static) {
    let _ = PICK_HANDLER.try_with(|slot| {
        if let Ok(mut guard) = slot.try_borrow_mut() {
            *guard = Some(Box::new(on_pick));
        }
    });

    let hwnd = WINDOW_HWND.with(|slot| *slot.borrow());
    let hwnd = match hwnd {
        Some(hwnd) if !hwnd.is_invalid() => hwnd,
        _ => match create() {
            Ok(hwnd) => hwnd,
            Err(err) => {
                crate::logging::log(&format!("settings: create failed: {err}"));
                return;
            }
        },
    };

    STATE.with(|slot| {
        if let Ok(mut guard) = slot.try_borrow_mut() {
            if let Some(state) = guard.as_mut() {
                // 打开时把高亮停在当前选择上，滚轮从这里开始走。
                state.hover = current
                    .as_deref()
                    .and_then(|code| {
                        EXTRA_LANGUAGES
                            .iter()
                            .position(|(c, _)| *c == code)
                            .map(|index| index + 1)
                    });
                state.current = current;
                state.scroll = 0;
                state.guard_hwnd =
                    unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }
                        .0 as isize;
            }
        }
    });

    let height = window_height();
    let (x, y) = clamp(anchor.x - WIDTH + 8, anchor.y);
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, WIDTH, height, SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = InvalidateRect(Some(hwnd), None, true);
        SetTimer(Some(hwnd), FOCUS_GUARD_TIMER, 500, None);
    }
}

pub fn hide() {
    if let Some(hwnd) = WINDOW_HWND.with(|slot| *slot.borrow()) {
        if !hwnd.is_invalid() {
            unsafe {
                let _ = KillTimer(Some(hwnd), FOCUS_GUARD_TIMER);
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

fn clamp(x: i32, y: i32) -> (i32, i32) {
    let height = window_height();
    unsafe {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = x.min(screen_w - WIDTH - 8).max(8);
        let y = if y + height + 8 > screen_h {
            (y - height - 30).max(8)
        } else {
            y + 8
        };
        (x, y)
    }
}

fn create() -> Result<HWND, String> {
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandle: {e}"))?;
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("MCTypeEnglishSettings"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("MCTypeEnglishSettings"),
            w!("MC-TypeEnglish 跟随语言"),
            WS_POPUP,
            0,
            0,
            WIDTH,
            PAD * 2 + TITLE_H + rows() as i32 * ROW_H,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| format!("CreateWindowExW: {e}"))?;

        let font = create_font(FONT_SIZE);
        STATE.with(|slot| {
            *slot.borrow_mut() = Some(State {
                current: None,
                hover: None,
                scroll: 0,
                font: font.0 as isize,
                guard_hwnd: 0,
            });
        });
        WINDOW_HWND.with(|slot| *slot.borrow_mut() = Some(hwnd));
        Ok(hwnd)
    }
}

unsafe fn create_font(height: i32) -> windows::Win32::Graphics::Gdi::HFONT {
    let face = "Microsoft YaHei UI";
    let mut logfont = LOGFONTW {
        lfHeight: -height,
        lfWeight: 400,
        lfCharSet: DEFAULT_CHARSET,
        lfOutPrecision: FONT_OUTPUT_PRECISION(OUT_TT_PRECIS.0 as u8),
        lfClipPrecision: CLIP_DEFAULT_PRECIS,
        lfQuality: CLEARTYPE_QUALITY,
        lfPitchAndFamily: FF_DONTCARE.0 | DEFAULT_PITCH.0,
        ..Default::default()
    };
    let units: Vec<u16> = face.encode_utf16().take(31).collect();
    for (slot, unit) in logfont.lfFaceName.iter_mut().zip(units) {
        *slot = unit;
    }
    CreateFontIndirectW(&logfont)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            let lparam = lparam.0 as u32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let scroll = STATE
                .try_with(|slot| slot.try_borrow().ok().and_then(|g| g.as_ref().map(|s| s.scroll)))
                .ok()
                .flatten()
                .unwrap_or(0);
            let row = (y - PAD - TITLE_H + scroll) / ROW_H;
            if row >= 0 && (row as usize) < rows() {
                let picked = language_at(row as usize);
                let handler = PICK_HANDLER
                    .try_with(|slot| slot.try_borrow().ok().and_then(|g| g.as_ref().map(|_| ())))
                    .ok()
                    .flatten();
                if handler.is_some() {
                    let _ = PICK_HANDLER.try_with(|slot| {
                        if let Ok(guard) = slot.try_borrow() {
                            if let Some(handler) = guard.as_ref() {
                                handler(picked.clone());
                            }
                        }
                    });
                }
            }
            hide();
            LRESULT(0)
        }
        // 鼠标滚轮：
        //   * 列表放不下时滚动列表；
        //   * 放得下时移动高亮（滚到哪一行，点一下就是哪一行）。
        // Win11 默认「悬停即可滚动非活动窗口」，所以这个不抢焦点的小窗也能收到。
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let notches = delta / 120;
            let changed = STATE
                .try_with(|slot| {
                    let Ok(mut guard) = slot.try_borrow_mut() else {
                        return false;
                    };
                    let Some(state) = guard.as_mut() else {
                        return false;
                    };
                    let limit = max_scroll();
                    if limit > 0 {
                        let next = (state.scroll - notches * ROW_H).clamp(0, limit);
                        let moved = next != state.scroll;
                        state.scroll = next;
                        return moved;
                    }
                    let last = rows() as i32 - 1;
                    let start = state.hover.map(|h| h as i32).unwrap_or(0);
                    let next = (start - notches).clamp(0, last);
                    let moved = state.hover != Some(next as usize);
                    state.hover = Some(next as usize);
                    moved
                })
                .unwrap_or(false);
            if changed {
                let _ = InvalidateRect(Some(hwnd), None, true);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == FOCUS_GUARD_TIMER => {
            let stale = STATE
                .try_with(|slot| {
                    let guard = slot.try_borrow().ok()?;
                    let state = guard.as_ref()?;
                    let current =
                        windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize;
                    Some(current != state.guard_hwnd)
                })
                .ok()
                .flatten()
                .unwrap_or(true);
            if stale {
                hide();
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    if dc.is_invalid() {
        return;
    }
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);

    let _ = STATE.try_with(|slot| {
        let Ok(guard) = slot.try_borrow() else {
            return;
        };
        let Some(state) = guard.as_ref() else {
            return;
        };

        let background = CreateSolidBrush(COLORREF(COLOR_BG));
        FillRect(dc, &rect, background);
        let _ = DeleteObject(HGDIOBJ(background.0));
        let border = CreateSolidBrush(COLORREF(COLOR_BORDER));
        let _ = FrameRect(dc, &rect, border);
        let _ = DeleteObject(HGDIOBJ(border.0));

        SetBkMode(dc, TRANSPARENT);
        let font = SelectObject(dc, HGDIOBJ(state.font as _));

        SetTextColor(dc, COLORREF(COLOR_TITLE));
        draw(dc, PAD, PAD + 4, TITLE);

        for row in 0..rows() {
            let top = PAD + TITLE_H + row as i32 * ROW_H - state.scroll;
            // 滚动后超出可视区的行不画
            if top + ROW_H < PAD + TITLE_H || top > rect.bottom - PAD {
                continue;
            }
            let selected = language_at(row) == state.current;
            let hovered = state.hover == Some(row);
            let row_rect = RECT {
                left: PAD - 4,
                right: rect.right - PAD + 4,
                top,
                bottom: top + ROW_H - 2,
            };
            if selected {
                let brush = CreateSolidBrush(COLORREF(COLOR_HILITE));
                FillRect(dc, &row_rect, brush);
                let _ = DeleteObject(HGDIOBJ(brush.0));
            } else if hovered {
                let brush = CreateSolidBrush(COLORREF(COLOR_HOVER));
                FillRect(dc, &row_rect, brush);
                let _ = DeleteObject(HGDIOBJ(brush.0));
            }
            SetTextColor(
                dc,
                COLORREF(if selected { COLOR_HILITE_TEXT } else { COLOR_TEXT }),
            );
            draw(dc, PAD + 6, top + 6, row_label(row));
        }

        SelectObject(dc, font);
    });

    let _ = EndPaint(hwnd, &ps);
}

unsafe fn draw(dc: HDC, x: i32, y: i32, text: &str) {
    if text.is_empty() {
        return;
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let _ = TextOutW(dc, x, y, &units);
}

/// 供其它模块确认窗口类是否已注册（调试用）。
pub fn class_name() -> &'static str {
    CLASS_NAME_STR
}

#[allow(dead_code)]
fn destroy() {
    if let Some(hwnd) = WINDOW_HWND.with(|slot| *slot.borrow()) {
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

