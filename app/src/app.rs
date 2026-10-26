//! Wiring: window classes, message loop, hook callback, tray menu, AI workers.
//!
//! Everything user-visible is single threaded. The only extra threads are the
//! AI requests, which post their answer back to this thread's message queue
//! (`WM_AI_RESULT`) instead of touching the engine directly.

use std::cell::RefCell;
use std::path::PathBuf;
use std::thread;

use windows::core::w;
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
    MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
    PostMessageW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow, TranslateMessage, MSG,
    SW_HIDE, SW_SHOWNORMAL, WM_APP, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONUP, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

use mcte_core::ai::{self, Suggestion};
use mcte_core::config::{self, Config};
use mcte_core::dict::Dict;
use mcte_core::engine::{Engine, Key, Outcome};
use mcte_core::glossary::Glossary;
use mcte_core::inject;
use mcte_core::keymap;
use mcte_core::logging::log;
use mcte_core::userdb::UserDb;
use mcte_core::window::CandidateWindow;

use crate::hook::{self, KeyboardHook};
use crate::tray::{Tray, TrayAction};

const WM_TRAY_ICON: u32 = WM_APP + 1;
const WM_AI_RESULT: u32 = WM_APP + 2;
const WM_AI_SPELL: u32 = WM_APP + 3;
const TIMER_AI_DEBOUNCE: usize = 1;
const TIMER_FLUSH_USERDB: usize = 2;
const HOTKEY_TOGGLE: i32 = 1;
/// 用户词库落盘间隔：够快（关掉进程也不丢太多），又不至于每个字都写盘。
const FLUSH_INTERVAL_MS: u32 = 5_000;

struct AiPayload {
    word: String,
    suggestions: Vec<Suggestion>,
}

/// AI 兜底生僻词的结果：拼音 -> 中文候选。
struct SpellPayload {
    composition: String,
    words: Vec<String>,
}

struct App {
    hwnd: HWND,
    engine: Engine,
    /// 只在终端类窗口里接管按键（`--terminal-only`）。
    /// 终端宿主不加载我们的 TSF 组件，用钩子补这一块；其它窗口让给 TSF，避免抢键。
    terminal_only: bool,
    /// 单独按下 Shift（中间没敲别的键）→ 抬起时切中/英。
    /// TSF 拿不到修饰键，所以 Shift 切换只在钩子版里可用。
    shift_alone: bool,
    window: CandidateWindow,
    tray: Tray,
    _hook: KeyboardHook,
    config_dir: PathBuf,
    data_dir: PathBuf,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|slot| {
        let mut guard = slot.borrow_mut();
        guard.as_mut().map(f)
    })
}

/// Load the engine's data files. Returns the paths so the tray can open them.
fn load_engine() -> Result<(Engine, PathBuf, PathBuf), String> {
    let config_dir = config::app_data_dir();
    let data_dir = config::data_dir();
    let cfg = Config::load_or_create(&config_dir);

    let dict_path = data_dir.join("dict.txt");
    let dict = Dict::load(&dict_path)
        .map_err(|e| format!("无法加载词库 {}: {e}", dict_path.display()))?;
    let glossary = Glossary::from_dir(&data_dir);
    if glossary.is_empty() {
        log("glossary.tsv missing or empty — only Chinese candidates will show");
    }
    let mut userdb = UserDb::open(&userdb_path(&config_dir));
    userdb.bind(&userdb_path(&config_dir));
    Ok((
        Engine::with_userdb(dict, glossary, cfg, userdb),
        config_dir,
        data_dir,
    ))
}

fn userdb_path(config_dir: &std::path::Path) -> PathBuf {
    config_dir.join("userdb.json")
}

pub fn run() -> Result<(), String> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let _instance = match single_instance() {
        Some(handle) => handle,
        None => {
            log("another instance is already running — exiting");
            return Ok(());
        }
    };

    let (engine, config_dir, data_dir) = load_engine()?;
    TRACE.store(
        std::env::args().any(|a| a == "--trace")
            || std::env::var("MCTE_TRACE").ok().as_deref() == Some("1"),
        std::sync::atomic::Ordering::Relaxed,
    );
    mcte_core::caret::set_trace(trace_on());
    mcte_core::window::set_trace(trace_on());
    let (dict_rows, user_words, spellings, bigrams) = engine.stats();
    log(&format!(
        "startup: {dict_rows} dict rows, {user_words} user words, {spellings} learned spellings, \
         {bigrams} bigram pairs, targets {:?}",
        engine.config().targets
    ));

    let hwnd = create_main_window()?;
    let window = CandidateWindow::create()?;
    let tray = Tray::add(hwnd, WM_TRAY_ICON, tray_tooltip(engine.ascii_mode()))?;
    let hook = KeyboardHook::install(keyboard_proc)?;

    let hotkey = parse_hotkey(&engine.config().toggle_hotkey);
    unsafe {
        if RegisterHotKey(Some(hwnd), HOTKEY_TOGGLE, hotkey, 'P' as u32).is_err() {
            log("RegisterHotKey failed — Ctrl+Alt+P toggle unavailable");
        }
    }

    // 鼠标点选：上层=中文、下层=英文，点中哪层就上屏哪层。
    mcte_core::window::set_click_handler(|row_index| {
        let text = with_app(|app| {
            let committed = app.engine.commit_row(row_index)?;
            app.refresh();
            Some(committed)
        })
        .flatten();
        if let Some(text) = text {
            log(&format!("click commit: {text:?}"));
            inject::type_text(&text);
        }
    });

    // 右上角「设置」：选择额外跟随的一种语言（英文固定跟随）。
    mcte_core::window::set_settings_handler(|| {
        let info = with_app(|app| {
            (
                app.window.anchor(),
                app.engine.config().extra_language.clone(),
                app.config_dir.clone(),
            )
        });
        let Some((anchor, current, dir)) = info else {
            return;
        };
        mcte_core::settings::open(anchor, current, move |picked| {
            with_app(|app| {
                let mut cfg = app.engine.config().clone();
                cfg.set_extra_language(picked.as_deref());
                if let Err(err) = cfg.save(&dir) {
                    log(&format!("settings: save failed: {err}"));
                }
                log(&format!(
                    "settings: extra language -> {:?}",
                    cfg.extra_language
                ));
                app.engine.set_config(cfg);
                app.refresh();
            });
        });
    });

    // 鼠标滚轮在候选窗上 = 翻页（向下下一页，向上上一页）。
    mcte_core::window::set_wheel_handler(|notches| {
        with_app(|app| {
            if app.engine.page_count() > 1 {
                let key = if notches > 0 {
                    Key::PageNext
                } else {
                    Key::PagePrev
                };
                app.engine.handle(key);
                app.refresh();
            }
        });
    });

    // 用户词库定期落盘：既不让学习结果随崩溃丢失太多，也不每个键都写盘。
    unsafe {
        SetTimer(Some(hwnd), TIMER_FLUSH_USERDB, FLUSH_INTERVAL_MS, None);
    }

    APP.with(|slot| {
        *slot.borrow_mut() = Some(App {
            hwnd,
            engine,
            terminal_only: std::env::args().any(|a| a == "--terminal-only"),
            shift_alone: false,
            window,
            tray,
            _hook: hook,
            config_dir,
            data_dir,
        });
    });
    log("ready: hook installed, tray icon added");

    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_TOGGLE);
        let _ = DestroyWindow(hwnd);
    }

    // 退出前落盘，别把刚学的词丢了。
    with_app(|app| {
        if let Err(err) = app.engine.flush_userdb() {
            log(&format!("flush userdb failed: {err}"));
        }
    });
    // Dropping the state unhooks the keyboard and removes the tray icon.
    APP.with(|slot| {
        slot.borrow_mut().take();
    });
    log("shutdown");
    Ok(())
}

/// `--demo <pinyin>`: show the candidate window at a fixed spot for 10 seconds.
/// Used by the screenshot verification, which needs the popup on a known pixel.
pub fn run_demo(input: &str) -> Result<(), String> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let (mut engine, _, _) = load_engine()?;
    for c in input.chars() {
        engine.handle(Key::Char(c));
    }
    // Let `--demo nihao 1.1` show the zoomed-in state used for committing.
    for extra in std::env::args().skip(3) {
        for c in extra.chars() {
            engine.handle(Key::Char(c));
        }
    }

    // 截图/点击验证用：把点中的行号写进日志
    mcte_core::window::set_click_handler(|row| {
        log(&format!("demo click: row {row}"));
    });

    let window = CandidateWindow::create()?;
    window.update_at(
        POINT { x: 240, y: 240 },
        &engine.preedit(),
        engine.rows(),
        engine.highlight(),
        engine.page(),
        engine.page_count(),
    );
    window.start_auto_quit(10_000);
    log(&format!("demo: {} rows for input {}", engine.rows().len(), input));

    // 演示模式下也把右上角「设置」接上，方便截图验证语言选择弹窗。
    let anchor = window.anchor();
    let current = engine.config().extra_language.clone();
    mcte_core::window::set_settings_handler(move || {
        log(&format!("demo: settings clicked, anchor={anchor:?}"));
        let current = current.clone();
        mcte_core::settings::open(anchor, current, |picked| {
            log(&format!("demo: picked extra language {picked:?}"));
        });
    });

    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

/// `MCTE_TRACE=1` 时把每个按键的归属写进日志（排查终端里不生效的问题用）。
static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn trace_on() -> bool {
    TRACE.load(std::sync::atomic::Ordering::Relaxed)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return hook::pass_through(code, wparam, lparam);
    }
    let event = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT);

    // Our own injected text must never be swallowed again.
    if event.dwExtraInfo == inject::MAGIC {
        return hook::pass_through(code, wparam, lparam);
    }
    let is_down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let is_up = matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP);
    if trace_on() && is_down {
        log(&format!(
            "trace: vk=0x{:02X} fg=\"{}\" terminal={} mods={} before={:?}",
            event.vkCode,
            foreground_class(),
            foreground_is_terminal(),
            modifiers_held(),
            with_app(|app| app.engine.preedit()).unwrap_or_default(),
        ));
    }

    // 终端模式：只在前台是终端类窗口时才接管；其它窗口交给 TSF 版，避免双份输入
    if with_app(|app| app.terminal_only).unwrap_or(false) && !foreground_is_terminal() {
        return hook::pass_through(code, wparam, lparam);
    }

    // 单独按一下 Shift 切中/英（按住 Shift 打大写字母不算）。
    // 注意：低层钩子收到的是 VK_LSHIFT(0xA0)/VK_RSHIFT(0xA1)，不是 VK_SHIFT(0x10)。
    if matches!(event.vkCode, 0x10 | 0xA0 | 0xA1) {
        if is_down {
            if !ctrl_alt_win_held() {
                with_app(|app| app.shift_alone = true);
            }
        } else if is_up {
            let toggled = with_app(|app| {
                let alone = app.shift_alone;
                app.shift_alone = false;
                if alone {
                    app.engine.toggle_ascii_mode();
                    app.update_tray();
                    let ascii = app.engine.ascii_mode();
                    let text = if ascii { "ABC 英文模式" } else { "中文模式" };
                    app.window.flash_tip("Shift", text, 700);
                }
                alone
            })
            .unwrap_or(false);
            if toggled {
                let ascii = with_app(|app| app.engine.ascii_mode()).unwrap_or(false);
                log(&format!(
                    "shift toggle -> {}",
                    if ascii { "ABC" } else { "中文" }
                ));
            }
        }
        return hook::pass_through(code, wparam, lparam);
    }
    if is_down {
        // 只要中间敲了别的键，Shift 就不再算「单独按」
        with_app(|app| app.shift_alone = false);
    }
    if !is_down {
        return hook::pass_through(code, wparam, lparam);
    }

    // Ctrl / Alt / Win 组合键留给应用程序（快捷键）
    if ctrl_alt_win_held() {
        return hook::pass_through(code, wparam, lparam);
    }
    // Shift: 如果按住 Shift 敲标点符号（如 ? ! : " 等），在中文模式下转中文标点；大写字母等穿透
    let shift = shift_held();
    let key = if shift {
        keymap::shifted_punctuation(event.vkCode).map(Key::Char)
    } else {
        keymap::map_vk(event.vkCode)
    };
    let Some(key) = key else {
        with_app(|app| {
            if app.window.is_showing_tip() {
                app.window.hide();
            }
        });
        return hook::pass_through(code, wparam, lparam);
    };

    let mut commit: Option<String> = None;
    let handled = with_app(|app| match app.engine.handle(key) {
        Outcome::Ignore => {
            if app.window.is_showing_tip() {
                app.window.hide();
            }
            if matches!(key, Key::Backspace | Key::Left | Key::Right | Key::Up | Key::Down) {
                app.engine.clear_recent_commits();
            }
            false
        }
        Outcome::Consumed => {
            app.refresh();
            true
        }
        Outcome::Commit(text) => {
            commit = Some(text);
            app.refresh();
            true
        }
    })
    .unwrap_or(false);

    if !handled {
        return hook::pass_through(code, wparam, lparam);
    }
    if trace_on() {
        log(&format!(
            "trace: handled vk=0x{:02X} commit={:?} after={:?}",
            event.vkCode,
            commit,
            with_app(|app| app.engine.preedit()).unwrap_or_default(),
        ));
    }
    if let Some(text) = commit {
        log(&format!("commit: {text:?}"));
        inject::type_text(&text);
    }
    LRESULT(1)
}

unsafe extern "system" fn main_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_HOTKEY => {
            with_app(|app| {
                app.engine.toggle_ascii_mode();
                app.update_tray();
                let ascii = app.engine.ascii_mode();
                let text = if ascii { "ABC 英文模式" } else { "中文模式" };
                app.window.flash_tip("Ctrl+Alt+P", text, 700);
                log(&format!(
                    "hotkey: mode -> {}",
                    if ascii { "ABC" } else { "中文" }
                ));
            });
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            let event = (lparam.0 as u32) & 0xFFFF;
            if event == WM_LBUTTONUP || event == WM_RBUTTONUP {
                let info = with_app(|app| {
                    (
                        app.engine.is_composing(),
                        {
                            let (dict_rows, user_words, _, _) = app.engine.stats();
                            format!(
                                "MC-TypeEnglish · {} · 词库 {dict_rows} · 已学 {user_words}",
                                if app.engine.ascii_mode() { "ABC" } else { "中文" }
                            )
                        },
                        format!(
                            "切换到{}模式",
                            if app.engine.ascii_mode() { "中文" } else { "ABC" }
                        ),
                    )
                });
                if let Some((_, status, mode_line)) = info {
                    let action = with_app(|app| app.tray.popup(&status, &mode_line));
                    match action {
                        Some(TrayAction::ToggleMode) => {
                            with_app(|app| {
                                app.engine.toggle_ascii_mode();
                                app.update_tray();
                                app.refresh();
                            });
                        }
                        Some(TrayAction::Reload) => {
                            with_app(|app| app.reload());
                        }
                        Some(TrayAction::OpenConfigDir) => {
                            with_app(|app| open_folder(&app.config_dir));
                        }
                        Some(TrayAction::OpenDataDir) => {
                            with_app(|app| open_folder(&app.data_dir));
                        }
                        Some(TrayAction::Quit) => unsafe { PostQuitMessage(0) },
                        _ => {}
                    }
                }
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_AI_DEBOUNCE => {
            with_app(|app| {
                unsafe {
                    let _ = KillTimer(Some(hwnd), TIMER_AI_DEBOUNCE);
                }
                app.schedule_ai();
                app.schedule_spelling_lookup();
            });
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_FLUSH_USERDB => {
            with_app(|app| {
                if app.engine.userdb().is_dirty() {
                    if let Err(err) = app.engine.flush_userdb() {
                        log(&format!("flush userdb failed: {err}"));
                    }
                }
            });
            LRESULT(0)
        }
        WM_AI_RESULT => {
            if lparam.0 != 0 {
                let payload = Box::from_raw(lparam.0 as *mut AiPayload);
                with_app(|app| {
                    let pairs: Vec<(String, String)> = payload
                        .suggestions
                        .iter()
                        .map(|s| (s.lang.clone(), s.text.clone()))
                        .collect();
                    log(&format!("ai[{}]: {:?}", payload.word, pairs));
                    app.engine.apply_ai_result(&payload.word, pairs);
                    app.refresh();
                });
            }
            LRESULT(0)
        }
        WM_AI_SPELL => {
            if lparam.0 != 0 {
                let payload = Box::from_raw(lparam.0 as *mut SpellPayload);
                with_app(|app| {
                    log(&format!("ai spell[{}]: {:?}", payload.composition, payload.words));
                    app.engine
                        .apply_spelling_result(&payload.composition, payload.words.clone());
                    app.refresh();
                });
            }
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

impl App {
    fn refresh(&mut self) {
        if self.engine.is_composing() {
            self.window.update(
                &self.engine.preedit(),
                self.engine.rows(),
                self.engine.highlight(),
                self.engine.page(),
                self.engine.page_count(),
            );
            unsafe {
                SetTimer(Some(self.hwnd), TIMER_AI_DEBOUNCE, 220, None);
            }
        } else {
            self.window.hide();
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_AI_DEBOUNCE);
            }
        }
    }

    fn update_tray(&self) {
        self.tray.set_tooltip(tray_tooltip(self.engine.ascii_mode()));
    }

    fn schedule_ai(&mut self) {
        let words = self.engine.pending_ai_words();
        if words.is_empty() {
            return;
        }
        for word in words {
            self.engine.mark_ai_requested(&word);
            let cfg = self.engine.config().clone();
            // HWND is a raw pointer and therefore not `Send`; carry the address
            // instead and rebuild the handle on the other side.
            let hwnd_addr = self.hwnd.0 as isize;
            log(&format!("translate: requesting {word}"));
            thread::spawn(move || {
                // 配置了 AI 就先问 AI（更准），否则直接用免费的网页翻译。
                let mut suggestions = Vec::new();
                if cfg.ai.enabled && !cfg.ai.api_key.trim().is_empty() {
                    match ai::translate(&cfg, &word) {
                        Ok(items) => suggestions = items,
                        Err(err) => log(&format!("ai[{word}]: {err}")),
                    }
                }
                if suggestions.is_empty() && cfg.web_translate {
                    let pairs = mcte_core::mt::translate_many(&word, &cfg.targets);
                    suggestions = pairs
                        .into_iter()
                        .map(|(lang, text)| Suggestion { lang, text })
                        .collect();
                }
                if suggestions.is_empty() {
                    return;
                }
                let payload = Box::new(AiPayload {
                    word,
                    suggestions,
                });
                let hwnd = HWND(hwnd_addr as *mut std::ffi::c_void);
                unsafe {
                    let _ = PostMessageW(
                        Some(hwnd),
                        WM_AI_RESULT,
                        WPARAM(0),
                        LPARAM(Box::into_raw(payload) as isize),
                    );
                }
            });
        }
    }

    /// 生僻词自动迭代：拼音在词库里查不到时，让 AI 给候选；结果落进用户词库，
    /// 下次完全离线也能出词。
    fn schedule_spelling_lookup(&mut self) {
        let Some(composition) = self.engine.pending_spelling_query() else {
            return;
        };
        if !self.engine.config().ai.enabled {
            return;
        }
        self.engine.mark_spelling_requested(&composition);
        let cfg = self.engine.config().clone();
        let hwnd_addr = self.hwnd.0 as isize;
        let query = composition.clone();
        log(&format!("ai spell: requesting {query}"));
        thread::spawn(move || match ai::spell(&cfg, &query) {
            Ok(words) if !words.is_empty() => {
                let payload = Box::new(SpellPayload {
                    composition: query,
                    words,
                });
                let hwnd = HWND(hwnd_addr as *mut std::ffi::c_void);
                unsafe {
                    let _ = PostMessageW(
                        Some(hwnd),
                        WM_AI_SPELL,
                        WPARAM(0),
                        LPARAM(Box::into_raw(payload) as isize),
                    );
                }
            }
            Ok(_) => {}
            Err(err) => log(&format!("ai spell[{query}]: {err}")),
        });
    }

    fn reload(&mut self) {
        let ascii = self.engine.ascii_mode();
        // 保留已学的用户词库：重载词库不该把个人习惯清零。
        let userdb = self.engine.userdb().clone();
        match load_engine() {
            Ok((mut engine, _, _)) => {
                engine.replace_userdb(userdb);
                engine.set_ascii_mode(ascii);
                let (dict_rows, user_words, spellings, bigrams) = engine.stats();
                log(&format!(
                    "reload: {dict_rows} dict rows, {user_words} user words, \
                     {spellings} spellings, {bigrams} bigrams"
                ));
                self.engine = engine;
                self.refresh();
                self.update_tray();
            }
            Err(err) => log(&format!("reload failed: {err}")),
        }
    }
}

fn tray_tooltip(ascii: bool) -> &'static str {
    if ascii {
        "MC-TypeEnglish（ABC 模式）"
    } else {
        "MC-TypeEnglish（中文模式）"
    }
}

/// 前台窗口是不是终端类（Windows Terminal / conhost / VS Code 终端 / mintty）。
fn foreground_is_terminal() -> bool {
    const TERMINAL_CLASSES: [&str; 4] = [
        "CASCADIA_HOSTING_WINDOW_CLASS", // Windows Terminal
        "ConsoleWindowClass",            // conhost
        "mintty",                        // Git Bash / Cygwin
        "VSCodeWindowClass",             // VS Code 集成终端
    ];
    let class = foreground_class();
    TERMINAL_CLASSES.iter().any(|name| *name == class)
}

fn foreground_class() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return String::new();
        }
        let mut buffer = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buffer);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..len as usize])
    }
}

fn ctrl_alt_win_held() -> bool {
    unsafe {
        let down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
            (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
        };
        down(VK_CONTROL) || down(VK_MENU) || down(VK_LWIN) || down(VK_RWIN)
    }
}

fn shift_held() -> bool {
    unsafe {
        (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0
    }
}

fn modifiers_held() -> bool {
    unsafe {
        let down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
            (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
        };
        down(VK_CONTROL) || down(VK_MENU) || down(VK_SHIFT) || down(VK_LWIN) || down(VK_RWIN)
    }
}


fn parse_hotkey(spec: &str) -> HOT_KEY_MODIFIERS {
    let mut mods = HOT_KEY_MODIFIERS(0);
    for part in spec.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" => mods |= MOD_WIN,
            _ => {}
        }
    }
    if mods.0 == 0 {
        mods = MOD_CONTROL | MOD_ALT;
    }
    mods
}

fn create_main_window() -> Result<HWND, String> {
    unsafe {
        let instance =
            GetModuleHandleW(None).map_err(|e| format!("GetModuleHandle failed: {e}"))?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(main_wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("MCTypeEnglishMain"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("MCTypeEnglishMain"),
            w!("MC-TypeEnglish"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(main) failed: {e}"))?;
        let _ = ShowWindow(hwnd, SW_HIDE);
        Ok(hwnd)
    }
}

fn single_instance() -> Option<HANDLE> {
    unsafe {
        // `Local\` works without SeCreateGlobalPrivilege (per logon session is
        // exactly the scope we want: one IME per interactive session).
        match CreateMutexW(None, true, w!("Local\\MC-TypeEnglish-IME")) {
            Ok(handle) => {
                if windows::Win32::Foundation::GetLastError()
                    == windows::Win32::Foundation::ERROR_ALREADY_EXISTS
                {
                    None
                } else {
                    Some(handle)
                }
            }
            Err(err) => {
                log(&format!("CreateMutexW failed: {err}"));
                // Better two instances than none: keep running.
                Some(HANDLE(std::ptr::null_mut()))
            }
        }
    }
}

fn open_folder(path: &PathBuf) {
    let units: Vec<u16> = path.to_string_lossy().encode_utf16().chain([0]).collect();
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            windows::core::PCWSTR(units.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}
