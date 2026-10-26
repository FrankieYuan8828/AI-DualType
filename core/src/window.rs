//! 候选窗：**横向卡片式**，一张卡片 = 上层中文 + 下层英文，最多 7 组一页，
//! 超过 7 组时左右显示 `‹` `›` 翻页（键位 `-` / `+` 或 PageUp / PageDown）。
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ nihao                                            1/2        │
//! │ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
//! │ │ 1  你好  │ │ 2  妳好  │ │ 3 你好啊 │ │ 4 你好吗 │  ...   │
//! │ │ 1.1 hello│ │ 2.1 hello│ │ 3.1 hello│ │ 4.1 hello│        │
//! │ └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! 字号/内边距按 Windows 11 自带输入法的观感来（中文 18px、译文 13px、卡片高约 54px）。

use std::cell::RefCell;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
    FrameRect, GetDC, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, TextOutW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    FF_DONTCARE, FONT_OUTPUT_PRECISION, HDC, HGDIOBJ, LOGFONTW, OUT_TT_PRECIS, PAINTSTRUCT,
    TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, KillTimer, PostQuitMessage, RegisterClassW, SetTimer,
    SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, MA_NOACTIVATE, SWP_NOACTIVATE,
    SW_SHOWNOACTIVATE, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::engine::Row;

const AUTOCLOSE_TIMER: usize = 99;
/// 焦点守卫：候选窗弹出后定期看一眼前台窗口有没有换人，换了就收起，
/// 免得宿主进程（浏览器、资源管理器）把窗口留在桌面上变成残影。
const FOCUS_GUARD_TIMER: usize = 98;
/// 提示自动隐藏定时器：中英切换提示等展示完毕后自动收起，绝不留残影
const TIP_AUTOHIDE_TIMER: usize = 97;
/// win32 的 WM_MOUSEWHEEL（这个版本的 windows crate 没有导出它，自己定义）
const WM_MOUSEWHEEL: u32 = 0x020A;

/// `--trace` 时把布局尺寸写进日志。
static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_trace(on: bool) {
    TRACE.store(on, std::sync::atomic::Ordering::Relaxed);
}

fn trace_on() -> bool {
    TRACE.load(std::sync::atomic::Ordering::Relaxed)
}

// ---- 度量（px）----
const PAD: i32 = 10;
const PREEDIT_H: i32 = 20;
const PREEDIT_GAP: i32 = 6;
const ZH_H: i32 = 26;
const SUB_H: i32 = 24; // 译文高度对齐中文！
const CELL_PAD_X: i32 = 12;
const CELL_PAD_Y: i32 = 6;
const CELL_H: i32 = CELL_PAD_Y * 2 + ZH_H + SUB_H;
const CELL_GAP: i32 = 8;
const CELL_MIN_W: i32 = 68;
const ARROW_W: i32 = 16;
const FONT_ZH: i32 = 18;
const FONT_SUB: i32 = 16; // 译文字号从 13px 调至 16px，与 18px 中文字号对齐、视觉字重平衡！
/// 一张卡片最多排出几条译文；再多就用 `+N` 概括。
const MAX_SUB_LINES: usize = 4;
/// 右上角设置按钮的文字与留白。
const SETTINGS_LABEL: &str = "设置";
const SETTINGS_PAD: i32 = 8;

// ---- 配色：现代化 Windows 11 Fluent 风格，精致雅白底色，柔和天青蓝高亮 ----
const COLOR_BG: u32 = 0x00F8_F9FA; // 候选窗底色（雅致浅灰白，非刺眼纯白）
const COLOR_BORDER: u32 = 0x00D8_DBDF; // 外边框精致中灰
const COLOR_CARD_BG: u32 = 0x00FF_FFFF; // 候选卡片纯白
const COLOR_CARD_BORDER: u32 = 0x00E2_E5E9; // 卡片未选中边框
const COLOR_DIVIDER: u32 = 0x00EE_F0F3; // 中文与译文间 1px 细线分隔，视觉清晰绝不混淆
const COLOR_TEXT: u32 = 0x001A_1A1A; // 中文字体深黑
const COLOR_SUB: u32 = 0x0033_3333; // 译文字体深灰黑（大字号更清晰）
const COLOR_HILITE: u32 = 0x00D7_7800; // 现代优雅天蓝 #0078D7 (BGR 0x00D7_7800)
const COLOR_HILITE_TEXT: u32 = 0x00FF_FFFF; // 选中白字
const COLOR_ARROW: u32 = 0x0078_7878;
const COLOR_ARROW_OFF: u32 = 0x00D0_D0D0;
const COLOR_SETTINGS_BG: u32 = 0x00ED_EFF2;

struct Cell {
    x: i32,
    w: i32,
    label: String,
    zh: String,
    /// 每条译文单独一行：(语言标签, 文本, rows 下标)。
    /// 不再用 `A / B` 挤在一行——用户要的是能一行一行选。
    subs: Vec<(String, String, usize)>,
    /// 被选中的译文行（None = 选中的是中文那一行）。
    selected: Option<usize>,
    parent_row: usize, // 点上层要提交的 rows 下标（中文）
    highlighted: bool,
}

struct PaintState {
    preedit: String,
    page: usize,
    page_count: usize,
    cells: Vec<Cell>,
    font_zh: isize,
    font_sub: isize,
    /// 卡片区高度（随译文行数变化），命中测试要用。
    card_h: i32,
    /// 弹窗时的前台窗口；它一变就说明用户切走了，窗口该收起来。
    guard_hwnd: isize,
    /// 右上角设置按钮占的宽度（命中测试和绘制共用）。
    settings_w: i32,
    /// 窗口客户区宽度，用来把设置按钮贴到右上角。
    window_w: i32,
    /// 输入会话锚点：同一次 composition 期间锁定位置，彻底解决输入窗漂移问题。
    anchor_origin: Option<windows::Win32::Foundation::POINT>,
    /// 是否为短暂临时提示（如中英切换提示）
    is_tip: bool,
}

thread_local! {
    static STATE: RefCell<Option<PaintState>> = const { RefCell::new(None) };
    /// 点击回调：参数是 `rows()` 里的下标（上层=父项，译文=某一行）
    static CLICK_HANDLER: RefCell<Option<Box<dyn Fn(usize)>>> = const { RefCell::new(None) };
    /// 右上角「设置」按钮的回调。
    static SETTINGS_HANDLER: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    /// 鼠标滚轮回调：参数是滚动的格数（向下为正）。
    static WHEEL_HANDLER: RefCell<Option<Box<dyn Fn(i32)>>> = const { RefCell::new(None) };
}

/// 注册滚轮回调：候选窗上滚轮 = 翻页。
pub fn set_wheel_handler(handler: impl Fn(i32) + 'static) {
    let _ = WHEEL_HANDLER.try_with(|slot| {
        if let Ok(mut guard) = slot.try_borrow_mut() {
            *guard = Some(Box::new(handler));
        }
    });
}

/// 注册「候选被鼠标点中」的回调。两个外壳各自把它接到自己的引擎上。
pub fn set_click_handler(handler: impl Fn(usize) + 'static) {
    let _ = CLICK_HANDLER.try_with(|slot| {
        if let Ok(mut guard) = slot.try_borrow_mut() {
            *guard = Some(Box::new(handler));
        }
    });
}

/// 注册「点了右上角设置」的回调（弹语言选择窗）。
pub fn set_settings_handler(handler: impl Fn() + 'static) {
    let _ = SETTINGS_HANDLER.try_with(|slot| {
        if let Ok(mut guard) = slot.try_borrow_mut() {
            *guard = Some(Box::new(handler));
        }
    });
}

fn settings_clicked(x: i32, y: i32) -> bool {
    STATE
        .try_with(|slot| {
            let Ok(guard) = slot.try_borrow() else {
                return false;
            };
            let Some(state) = guard.as_ref() else {
                return false;
            };
            if state.settings_w <= 0 {
                return false;
            }
            // 右上角那一带：顶部一行，宽度就是「设置」两个字加上留白
            y >= 0
                && y <= PAD + PREEDIT_H
                && x >= settings_left(state) - 4
                && x <= settings_left(state) + state.settings_w + 4
        })
        .ok()
        .unwrap_or(false)
}

fn settings_left(state: &PaintState) -> i32 {
    state.window_w - PAD - state.settings_w
}

/// 命中测试：坐标落在哪一行就返回对应行号（中文行或某条译文）。
fn hit_test(x: i32, y: i32) -> Option<usize> {
    STATE
        .try_with(|slot| {
            let Ok(guard) = slot.try_borrow() else {
                return None;
            };
            let state = guard.as_ref()?;
            let top = PAD + PREEDIT_H + PREEDIT_GAP;
            if y < top || y > top + state.card_h {
                return None;
            }
            for cell in &state.cells {
                if x >= cell.x && x < cell.x + cell.w {
                    if y < top + CELL_PAD_Y + ZH_H {
                        return Some(cell.parent_row);
                    }
                    let line = (y - (top + CELL_PAD_Y + ZH_H + 2)) / SUB_H;
                    return cell
                        .subs
                        .get(line as usize)
                        .map(|(_, _, row)| *row)
                        .or(Some(cell.parent_row));
                }
            }
            None
        })
        .ok()
        .flatten()
}

pub struct CandidateWindow {
    hwnd: HWND,
}

impl CandidateWindow {
    pub fn create() -> Result<Self, String> {
        unsafe {
            let instance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .map_err(|e| format!("GetModuleHandle failed: {e}"))?;
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("MCTypeEnglishCandidates"),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("MCTypeEnglishCandidates"),
                w!("MC-TypeEnglish"),
                WS_POPUP,
                0,
                0,
                320,
                90,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .map_err(|e| format!("CreateWindowExW failed: {e}"))?;

            let font_zh = create_font(FONT_ZH, 500);
            let font_sub = create_font(FONT_SUB, 400);
            STATE.with(|s| {
                *s.borrow_mut() = Some(PaintState {
                    preedit: String::new(),
                    page: 0,
                    page_count: 1,
                    cells: Vec::new(),
                    font_zh: font_zh.0 as isize,
                    font_sub: font_sub.0 as isize,
                    card_h: CELL_H,
                    guard_hwnd: 0,
                    settings_w: 0,
                    window_w: 0,
                    anchor_origin: None,
                    is_tip: false,
                });
            });

            Ok(Self { hwnd })
        }
    }

    /// 当前是否处于短暂提示状态（如切中/英 toast）。
    pub fn is_showing_tip(&self) -> bool {
        STATE
            .try_with(|s| s.borrow().as_ref().map(|st| st.is_tip).unwrap_or(false))
            .unwrap_or(false)
    }

    /// 弹出一条自动消失的短暂模式提示（toast），并在指定毫秒后自动收起（绝不留残影）。
    pub fn flash_tip(&self, title: &str, text: &str, duration_ms: u32) {
        let row = Row {
            label: String::new(),
            text: text.to_string(),
            depth: 0,
            lang: String::new(),
        };
        let pt = crate::caret::caret_point();
        self.update_at(pt, title, std::slice::from_ref(&row), 0, 0, 1);
        let _ = STATE.try_with(|s| {
            if let Ok(mut guard) = s.try_borrow_mut() {
                if let Some(state) = guard.as_mut() {
                    state.is_tip = true;
                }
            }
        });
        unsafe {
            let _ = SetTimer(Some(self.hwnd), TIP_AUTOHIDE_TIMER, duration_ms, None);
        }
    }

    /// 刷新并跟随光标定位（输入期间锁定锚点，彻底防止输入窗跳跃与漂移）。
    pub fn update(
        &self,
        preedit: &str,
        rows: &[Row],
        highlight: usize,
        page: usize,
        page_count: usize,
    ) {
        let origin = STATE.with(|s| {
            let mut guard = s.borrow_mut();
            if let Some(state) = guard.as_mut() {
                let fg = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize };
                if let Some(pt) = state.anchor_origin {
                    if state.guard_hwnd == fg && fg != 0 {
                        return pt;
                    }
                }
                let pt = crate::caret::caret_point();
                state.anchor_origin = Some(pt);
                pt
            } else {
                crate::caret::caret_point()
            }
        });
        self.update_at(origin, preedit, rows, highlight, page, page_count);
    }

    /// 在指定位置刷新（`--demo` / 截图验证用）。
    pub fn update_at(
        &self,
        origin: windows::Win32::Foundation::POINT,
        preedit: &str,
        rows: &[Row],
        highlight: usize,
        page: usize,
        page_count: usize,
    ) {
        let (width, height) = unsafe {
            let size = STATE.with(|s| {
                let mut guard = s.borrow_mut();
                let Some(state) = guard.as_mut() else {
                    return (0, 0, 0, 0, 0);
                };
                state.is_tip = false;
                state.preedit = preedit.to_string();
                state.page = page;
                state.page_count = page_count.max(1);
                state.cells = layout(rows, highlight);
                state.guard_hwnd = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize;
                let (w, h) = measure(state);
                let cells = state.cells.len();
                let lines = state.cells.iter().map(|c| c.subs.len()).max().unwrap_or(0);
                (w, h, state.card_h, cells, lines)
            });
            if trace_on() && size.2 > 0 {
                crate::logging::log(&format!(
                    "ui: {}x{} card_h={} cells={} max_subs={} rows={}",
                    size.0, size.1, size.2, size.3, size.4, rows.len()
                ));
            }
            (size.0, size.1)
        };

        if width == 0 || height == 0 {
            crate::logging::log(&format!(
                "ui: hide (rows={}, cells={}, page={}/{})",
                rows.len(),
                STATE.try_with(|s| s.borrow().as_ref().map(|st| st.cells.len()).unwrap_or(0)).unwrap_or(0),
                page + 1,
                page_count
            ));
            self.hide();
            return;
        }


        let (x, y) = clamp_to_screen(origin.x, origin.y, width, height);
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIP_AUTOHIDE_TIMER);
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            );
            if trace_on() {
                let mut got = windows::Win32::Foundation::RECT::default();
                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut got);
                crate::logging::log(&format!(
                    "ui: asked {}x{} at {}x{}, got {}x{}",
                    width,
                    height,
                    x,
                    y,
                    got.right - got.left,
                    got.bottom - got.top
                ));
            }
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            round_corners(self.hwnd, width, height);
            let _ = InvalidateRect(Some(self.hwnd), None, true);
            // 起一个 500ms 的焦点守卫，见 `FOCUS_GUARD_TIMER` 的说明。
            SetTimer(Some(self.hwnd), FOCUS_GUARD_TIMER, 500, None);
        }
    }

    /// 窗口右上角的屏幕坐标，设置弹窗贴在它旁边。
    pub fn anchor(&self) -> POINT {
        let mut rect = RECT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rect);
        }
        POINT {
            x: rect.right,
            y: rect.top,
        }
    }

    /// `--demo` 用它自动退出，免得截图脚本还要杀进程。
    pub fn start_auto_quit(&self, milliseconds: u32) {
        unsafe {
            SetTimer(Some(self.hwnd), AUTOCLOSE_TIMER, milliseconds, None);
        }
    }

    pub fn hide(&self) {
        let _ = STATE.try_with(|s| {
            if let Ok(mut guard) = s.try_borrow_mut() {
                if let Some(state) = guard.as_mut() {
                    state.anchor_origin = None;
                    state.is_tip = false;
                }
            }
        });
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIP_AUTOHIDE_TIMER);
            let _ = KillTimer(Some(self.hwnd), FOCUS_GUARD_TIMER);
            let _ = ShowWindow(self.hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        // 进程退出阶段 TLS 可能已析构，`with` 会 panic —— 这里允许它失败。
        let _ = STATE.try_with(|s| {
            if let Ok(mut guard) = s.try_borrow_mut() {
                if let Some(state) = guard.take() {
                    unsafe {
                        let _ = DeleteObject(HGDIOBJ(state.font_zh as _));
                        let _ = DeleteObject(HGDIOBJ(state.font_sub as _));
                    }
                }
            }
        });
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
        }
    }
}

unsafe fn create_font(height: i32, weight: i32) -> windows::Win32::Graphics::Gdi::HFONT {
    let face = "Microsoft YaHei UI";
    let mut logfont = LOGFONTW {
        lfHeight: -height,
        lfWeight: weight,
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

unsafe fn text_width(dc: HDC, font: isize, text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }
    let previous = SelectObject(dc, HGDIOBJ(font as _));
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    let ok = GetTextExtentPoint32W(dc, &units, &mut size);
    SelectObject(dc, previous);
    if ok.as_bool() {
        size.cx
    } else {
        text.chars().map(|c| if c.is_ascii() { 8 } else { 18 }).sum()
    }
}

/// 把 `rows`（父项 + 译文）折叠成横向卡片。
///
/// 一张卡片 = 中文一行 + 每条译文各一行；不再把多条译文用 ` / ` 挤成一行，
/// 这样连按同一个数字就能一行一行往下选。
fn layout(rows: &[Row], highlight: usize) -> Vec<Cell> {
    let mut cells: Vec<Cell> = Vec::new();
    let mut index_of_parent: Vec<usize> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row.depth == 0 {
            index_of_parent.push(cells.len());
            cells.push(Cell {
                x: 0,
                w: 0,
                label: row.label.clone(),
                zh: row.text.clone(),
                subs: Vec::new(),
                selected: None,
                parent_row: row_index,
                highlighted: row_index == highlight,
            });
        } else if let Some(&cell_index) = index_of_parent.last() {
            let cell = &mut cells[cell_index];
            cell.subs
                .push((row.lang.clone(), row.text.clone(), row_index));
            if row_index == highlight {
                cell.selected = Some(cell.subs.len() - 1);
                cell.highlighted = false;
            }
        }
    }
    cells
}

/// 译文行的显示文本：`英文 hello`。语言标签让人一眼知道选的是哪种语言。
fn sub_line(lang: &str, text: &str) -> String {
    if lang.is_empty() {
        text.to_string()
    } else {
        format!("{lang} {text}")
    }
}


unsafe fn measure(state: &mut PaintState) -> (i32, i32) {
    let dc = GetDC(None);
    if dc.is_invalid() || state.cells.is_empty() {
        if !dc.is_invalid() {
            ReleaseDC(None, dc);
        }
        return (0, 0);
    }

    // 翻页时左右各留一条箭头带，否则箭头会压在第 1/7 张卡上
    let arrow_band = if state.page_count > 1 { ARROW_W + 6 } else { 0 };
    let mut cursor = PAD + arrow_band;
    let arrows = arrow_band * 2;
    let mut preedit_w = text_width(dc, state.font_sub, &state.preedit);
    let page_text = if state.page_count > 1 {
        format!("{}/{}", state.page + 1, state.page_count)
    } else {
        String::new()
    };
    preedit_w += text_width(dc, state.font_sub, &page_text) + 24;
    let settings_w = text_width(dc, state.font_sub, SETTINGS_LABEL) + SETTINGS_PAD;
    state.settings_w = settings_w;

    // 卡片宽度按最长的一行算；高度按这一页里译文最多的那张卡算，
    // 让所有卡片底边对齐，看起来是一排整齐的列。
    let mut most_lines = 0usize;
    for cell in state.cells.iter_mut() {
        let zh_line = format!("{} {}", cell.label, cell.zh);
        let mut widest = text_width(dc, state.font_zh, &zh_line);
        let visible = cell.subs.len().min(MAX_SUB_LINES);
        for (index, (lang, text, _)) in cell.subs.iter().take(visible).enumerate() {
            let mut line = sub_line(lang, text);
            if index + 1 == visible && cell.subs.len() > MAX_SUB_LINES {
                line.push_str(&format!(" +{}", cell.subs.len() - MAX_SUB_LINES));
            }
            widest = widest.max(text_width(dc, state.font_sub, &line));
        }
        cell.w = (widest + CELL_PAD_X * 2).max(CELL_MIN_W);
        cell.x = cursor;
        cursor += cell.w + CELL_GAP;
        most_lines = most_lines.max(visible);
    }
    ReleaseDC(None, dc);

    state.card_h = CELL_PAD_Y * 2 + ZH_H + most_lines as i32 * SUB_H;

    // 左边 PAD + 箭头带 + 卡片 + 右边箭头带 + PAD
    let strip_w = PAD + (cursor - CELL_GAP - PAD - arrow_band) + arrows + PAD;
    let width = strip_w.max(preedit_w + PAD * 2 + settings_w + 8).max(220);
    let height = PAD * 2 + PREEDIT_H + PREEDIT_GAP + state.card_h;
    state.window_w = width;
    (width, height)
}

/// 卡片式圆角：Win11 用 DWM 的圆角偏好（带抗锯齿），旧系统退回 `SetWindowRgn`。
unsafe fn round_corners(hwnd: HWND, width: i32, height: i32) {
    // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: windows::Win32::Foundation::HWND,
            dwattribute: u32,
            pvattribute: *const core::ffi::c_void,
            cbattribute: u32,
        ) -> i32;
    }
    let preference: i32 = 2;
    let hr = DwmSetWindowAttribute(
        hwnd,
        33,
        &preference as *const i32 as *const core::ffi::c_void,
        std::mem::size_of::<i32>() as u32,
    );
    if hr < 0 {
        // Win10 及更早：用窗口区域裁圆角
        let radius = 12;
        let region = windows::Win32::Graphics::Gdi::CreateRoundRectRgn(
            0,
            0,
            width + 1,
            height + 1,
            radius,
            radius,
        );
        if !region.is_invalid() {
            windows::Win32::Graphics::Gdi::SetWindowRgn(hwnd, Some(region), true);
        }
    }
}

fn clamp_to_screen(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    unsafe {
        let screen_w = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
        );
        let screen_h = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
        );
        let x = x.min(screen_w - width - 8).max(8);
        let y = if y + height + 8 > screen_h {
            (y - height - 28).max(8)
        } else {
            y + 8
        };
        (x, y)
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            let is_tip = STATE
                .try_with(|s| s.borrow().as_ref().map(|st| st.is_tip).unwrap_or(false))
                .unwrap_or(false);
            if is_tip {
                let _ = KillTimer(Some(hwnd), TIP_AUTOHIDE_TIMER);
                let _ = KillTimer(Some(hwnd), FOCUS_GUARD_TIMER);
                let _ = STATE.try_with(|s| {
                    if let Ok(mut guard) = s.try_borrow_mut() {
                        if let Some(state) = guard.as_mut() {
                            state.is_tip = false;
                            state.anchor_origin = None;
                        }
                    }
                });
                let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
                return LRESULT(0);
            }
            let lparam = lparam.0 as u32;
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if settings_clicked(x, y) {
                let _ = SETTINGS_HANDLER.try_with(|slot| {
                    if let Ok(guard) = slot.try_borrow() {
                        if let Some(handler) = guard.as_ref() {
                            handler();
                        }
                    }
                });
                return LRESULT(0);
            }
            if let Some(row) = hit_test(x, y) {
                let _ = CLICK_HANDLER.try_with(|slot| {
                    if let Ok(guard) = slot.try_borrow() {
                        if let Some(handler) = guard.as_ref() {
                            handler(row);
                        }
                    }
                });
            }
            LRESULT(0)
        }
        // 鼠标滚轮：向下 = 下一页，向上 = 上一页
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let notches = delta / 120;
            if notches != 0 {
                let _ = WHEEL_HANDLER.try_with(|slot| {
                    if let Ok(guard) = slot.try_borrow() {
                        if let Some(handler) = guard.as_ref() {
                            handler(-notches);
                        }
                    }
                });
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == AUTOCLOSE_TIMER => {
            let _ = KillTimer(Some(hwnd), AUTOCLOSE_TIMER);
            PostQuitMessage(0);
            LRESULT(0)
        }
        // 模式提示超时：自动隐藏，绝不留残影
        WM_TIMER if wparam.0 == TIP_AUTOHIDE_TIMER => {
            let _ = KillTimer(Some(hwnd), TIP_AUTOHIDE_TIMER);
            let _ = KillTimer(Some(hwnd), FOCUS_GUARD_TIMER);
            let _ = STATE.try_with(|s| {
                if let Ok(mut guard) = s.try_borrow_mut() {
                    if let Some(state) = guard.as_mut() {
                        state.is_tip = false;
                        state.anchor_origin = None;
                    }
                }
            });
            let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
            LRESULT(0)
        }
        // 焦点守卫：前台窗口换人就把候选窗收起来，绝不留残影。
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
                let _ = KillTimer(Some(hwnd), FOCUS_GUARD_TIMER);
                let _ = KillTimer(Some(hwnd), TIP_AUTOHIDE_TIMER);
                let _ = STATE.try_with(|s| {
                    if let Ok(mut guard) = s.try_borrow_mut() {
                        if let Some(state) = guard.as_mut() {
                            state.is_tip = false;
                            state.anchor_origin = None;
                        }
                    }
                });
                let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
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
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);

    let _ = STATE.try_with(|s| {
        let Ok(guard) = s.try_borrow() else {
            return;
        };
        let Some(state) = guard.as_ref() else {
            return;
        };

        let background = CreateSolidBrush(COLORREF(COLOR_BG));
        FillRect(dc, &rect, background);
        let _ = DeleteObject(HGDIOBJ(background.0));
        let border = CreateSolidBrush(COLORREF(COLOR_BORDER));
        let _ = windows::Win32::Graphics::Gdi::FrameRect(dc, &rect, border);
        let _ = DeleteObject(HGDIOBJ(border.0));

        SetBkMode(dc, TRANSPARENT);

        // 顶部：拼音 + 页码 + 右上角设置按钮
        let sub_font = SelectObject(dc, HGDIOBJ(state.font_sub as _));
        SetTextColor(dc, COLORREF(COLOR_SUB));
        draw_text(dc, PAD, PAD + 3, &state.preedit);
        let settings_left = settings_left(state);
        if state.page_count > 1 {
            let page_text = format!("{}/{}", state.page + 1, state.page_count);
            let w = text_width(dc, state.font_sub, &page_text);
            SetTextColor(dc, COLORREF(COLOR_ARROW));
            draw_text(dc, settings_left - 10 - w, PAD + 3, &page_text);
        }
        // 设置按钮：一个浅色圆角块 + 「设置」两个字，点了弹语言选择
        if state.settings_w > 0 {
            let button = RECT {
                left: settings_left - 4,
                right: settings_left + state.settings_w - 4,
                top: PAD - 1,
                bottom: PAD + PREEDIT_H,
            };
            let brush = CreateSolidBrush(COLORREF(COLOR_SETTINGS_BG));
            FillRect(dc, &button, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
            SetTextColor(dc, COLORREF(COLOR_ARROW));
            draw_text(dc, settings_left + 2, PAD + 3, SETTINGS_LABEL);
        }
        SelectObject(dc, sub_font);

        // 翻页箭头
        if state.page_count > 1 {
            let arrow_top = PAD + PREEDIT_H + PREEDIT_GAP + state.card_h / 2 - 9;
            SetTextColor(
                dc,
                COLORREF(if state.page > 0 {
                    COLOR_ARROW
                } else {
                    COLOR_ARROW_OFF
                }),
            );
            draw_text(dc, PAD - 2, arrow_top, "‹");
            SetTextColor(
                dc,
                COLORREF(if state.page + 1 < state.page_count {
                    COLOR_ARROW
                } else {
                    COLOR_ARROW_OFF
                }),
            );
            let right = rect.right - PAD - 10;
            draw_text(dc, right, arrow_top, "›");
        }

        let top = PAD + PREEDIT_H + PREEDIT_GAP;
        for cell in &state.cells {
            let cell_rect = RECT {
                left: cell.x,
                right: cell.x + cell.w,
                top,
                bottom: top + state.card_h,
            };

            // 卡片纯白底色，细腻边框
            let card_bg = CreateSolidBrush(COLORREF(COLOR_CARD_BG));
            FillRect(dc, &cell_rect, card_bg);
            let _ = DeleteObject(HGDIOBJ(card_bg.0));
            let card_border = CreateSolidBrush(COLORREF(COLOR_CARD_BORDER));
            let _ = FrameRect(dc, &cell_rect, card_border);
            let _ = DeleteObject(HGDIOBJ(card_border.0));

            // 如果该卡片有译文，在中英文之间画一条浅色分隔线，将中文与译文清晰分层
            if !cell.subs.is_empty() {
                let div_y = top + CELL_PAD_Y + ZH_H;
                let div_rect = RECT {
                    left: cell.x + 4,
                    right: cell.x + cell.w - 4,
                    top: div_y,
                    bottom: div_y + 1,
                };
                let div_brush = CreateSolidBrush(COLORREF(COLOR_DIVIDER));
                FillRect(dc, &div_rect, div_brush);
                let _ = DeleteObject(HGDIOBJ(div_brush.0));
            }

            if cell.highlighted {
                // 选中中文：只把中文胶囊涂蓝，译文行仍是白底深字，层次分明
                let band = RECT {
                    left: cell.x + 2,
                    right: cell.x + cell.w - 2,
                    top: top + 2,
                    bottom: top + CELL_PAD_Y + ZH_H - 1,
                };
                let brush = CreateSolidBrush(COLORREF(COLOR_HILITE));
                FillRect(dc, &band, brush);
                let _ = DeleteObject(HGDIOBJ(brush.0));
            }

            // 上层：`1 你好`（序号用子字号画在左边，中文用主字号）
            let label_font = SelectObject(dc, HGDIOBJ(state.font_sub as _));
            SetTextColor(
                dc,
                COLORREF(if cell.highlighted {
                    COLOR_HILITE_TEXT
                } else {
                    COLOR_TEXT
                }),
            );
            let label_w = text_width(dc, state.font_sub, &cell.label);
            let zh_w = text_width(dc, state.font_zh, &cell.zh);
            let content_w = label_w + 6 + zh_w;
            let start_x = cell.x + (cell.w - content_w) / 2;
            draw_text(dc, start_x, top + CELL_PAD_Y + 4, &cell.label);
            SelectObject(dc, label_font);

            let zh_font = SelectObject(dc, HGDIOBJ(state.font_zh as _));
            SetTextColor(
                dc,
                COLORREF(if cell.highlighted {
                    COLOR_HILITE_TEXT
                } else {
                    COLOR_TEXT
                }),
            );
            draw_text(dc, start_x + label_w + 6, top + CELL_PAD_Y, &cell.zh);
            SelectObject(dc, zh_font);

            // 译文：一条一行，被选中的那行反色高亮
            let sub_font = SelectObject(dc, HGDIOBJ(state.font_sub as _));
            let visible = cell.subs.len().min(MAX_SUB_LINES);
            for (index, (lang, text, _)) in cell.subs.iter().take(visible).enumerate() {
                let mut line = sub_line(lang, text);
                if index + 1 == visible && cell.subs.len() > MAX_SUB_LINES {
                    line.push_str(&format!(" +{}", cell.subs.len() - MAX_SUB_LINES));
                }
                let line_top = top + CELL_PAD_Y + ZH_H + 2 + index as i32 * SUB_H;
                let chosen = cell.selected == Some(index);
                if chosen {
                    let line_rect = RECT {
                        left: cell.x + 2,
                        right: cell.x + cell.w - 2,
                        top: line_top - 1,
                        bottom: line_top + SUB_H - 1,
                    };
                    let brush = CreateSolidBrush(COLORREF(COLOR_HILITE));
                    FillRect(dc, &line_rect, brush);
                    let _ = DeleteObject(HGDIOBJ(brush.0));
                }
                SetTextColor(
                    dc,
                    COLORREF(if chosen { COLOR_HILITE_TEXT } else { COLOR_SUB }),
                );
                let line_w = text_width(dc, state.font_sub, &line);
                let line_x = cell.x + (cell.w - line_w) / 2;
                draw_text(dc, line_x, line_top + 1, &line);
            }
            SelectObject(dc, sub_font);
        }
    });

    let _ = EndPaint(hwnd, &ps);
}

unsafe fn draw_text(dc: HDC, x: i32, y: i32, text: &str) {
    if text.is_empty() {
        return;
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let _ = TextOutW(dc, x, y, &units);
}

/// 供调试工具查看窗口类名。
pub const CLASS_NAME: PCWSTR = w!("MCTypeEnglishCandidates");

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, text: &str, depth: u8, lang: &str) -> Row {
        Row {
            label: label.to_string(),
            text: text.to_string(),
            depth,
            lang: lang.to_string(),
        }
    }

    /// 一张卡片里，每条译文各占一行，顺序不能和语言标签错位。
    #[test]
    fn each_translation_becomes_its_own_line() {
        let rows = vec![
            row("1", "你好", 0, ""),
            row("1.1", "hello", 1, "英文"),
            row("1.2", "hallo", 1, "德语"),
            row("1.3", "こんにちは", 1, "日语"),
        ];
        let cells = layout(&rows, 0);
        assert_eq!(cells.len(), 1);
        let cell = &cells[0];
        assert_eq!(
            cell.subs,
            vec![
                ("英文".into(), "hello".into(), 1),
                ("德语".into(), "hallo".into(), 2),
                ("日语".into(), "こんにちは".into(), 3),
            ]
        );
        assert!(cell.highlighted, "高亮在中文行时整张卡片高亮");
        assert_eq!(cell.selected, None);
    }

    /// 连按数字把高亮挪到某条译文时，高亮必须落在**那条**译文上，
    /// 而不是固定在英文那一行（这正是之前「按 1 三次却看到日文配 hallo」的来源）。
    #[test]
    fn selected_translation_matches_its_language() {
        let rows = vec![
            row("1", "你好", 0, ""),
            row("1.1", "hello", 1, "英文"),
            row("1.2", "hallo", 1, "德语"),
            row("1.3", "こんにちは", 1, "日语"),
        ];
        // highlight = 3 是「日语 こんにちは」那一行
        let cells = layout(&rows, 3);
        assert_eq!(cells[0].selected, Some(2), "第三条译文（日语）被选中");
        assert!(!cells[0].highlighted, "选中译文时中文行不再整块高亮");
        assert_eq!(sub_line("日语", "こんにちは"), "日语 こんにちは");
    }

    #[test]
    fn candidate_without_translations_has_no_sub_lines() {
        let rows = vec![row("1", "痕迹", 0, "")];
        let cells = layout(&rows, 0);
        assert!(cells[0].subs.is_empty());
        assert_eq!(cells[0].selected, None);
    }
}
