//! TSF 文本服务的两个 COM 对象：
//!
//! * `TextService` —— `ITfTextInputProcessor(Ex)`，负责激活/去激活、加载引擎；
//! * `KeySink`     —— `ITfKeyEventSink`，负责收键、跑状态机、把文字插进文档。
//!
//! 拆成两个对象是有意为之：`ITfKeystrokeMgr::AdviseKeyEventSink` 需要传"自己"这个
//! `ITfKeyEventSink` 指针，而在 `#[implement]` 生成的包装类型里拿不到外层 COM 指针；
//! 拆开后 `KeySink` 是独立对象，通过 `Arc<Mutex<State>>` 与本对象共享同一份引擎状态。
//!
//! 注意 windows-rs 0.6x 的写法：接口 trait 要实现在宏生成的 `<类型>_Impl` 上，
//! 内部状态通过 `IUnknownImpl::get_impl()` 访问。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use windows::core::{implement, Interface, IUnknownImpl, Ref, Result as WinResult, BOOL};
use windows::Win32::Foundation::{E_FAIL, FALSE, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection, ITfKeyEventSink,
    ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfTextInputProcessor, ITfTextInputProcessor_Impl,
    ITfLangBarItemButton, ITfLangBarItemMgr, ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl,
    ITfThreadMgr, INSERT_TEXT_AT_SELECTION_FLAGS, TF_ES_READWRITE, TF_ES_SYNC, TF_MOD_ALT,
    TF_MOD_CONTROL, TF_PRESERVEDKEY,
};

use mcte_core::config::{self, Config};
use mcte_core::dict::Dict;
use mcte_core::engine::{Engine, Key, Outcome};
use mcte_core::glossary::Glossary;
use mcte_core::keymap;
use mcte_core::logging::log;
use mcte_core::userdb::UserDb;
use mcte_core::window::CandidateWindow;

/// 每上屏这么多次就把用户词库落一次盘（TSF 没有自己的消息循环，不能靠定时器）。
const FLUSH_EVERY_COMMITS: u32 = 8;

/// 是否已经因为「宿主不支持 TIP 插入」而回退过（只为少写日志）。
static FALLBACK_USED: AtomicBool = AtomicBool::new(false);

/// 单独按一下 Shift 就切换中/英（与微软拼音一致）；按住 Shift 打大写字母不算。
const VK_SHIFT_CODE: u32 = 0x10;
/// 中/英切换的保留键：Ctrl+Alt+P（与钩子版快捷键一致）
const VK_P_CODE: u32 = 0x50;
const GUID_TOGGLE: windows::core::GUID =
    windows::core::GUID::from_u128(0x9e1a4d0c_2b77_4b1f_a0d3_5c8f6e2b7a41);

struct State {
    thread_mgr: Option<ITfThreadMgr>,
    client_id: u32,
    engine: Option<Engine>,
    /// 任务栏输入指示器的语言栏按钮（没有它，系统只能画通用占位图标）
    langbar: Option<ITfLangBarItemButton>,
    langbar_mgr: Option<ITfLangBarItemMgr>,
    commits_since_flush: u32,
    /// Shift 按下后没有别的键介入 → 抬起时切换中/英
    shift_alone: bool,
    /// 最近一次按键的编辑上下文：鼠标点选上屏时要用它
    last_context: Option<ITfContext>,
}

impl State {
    fn new() -> Self {
        Self {
            thread_mgr: None,
            client_id: 0,
            engine: None,
            langbar: None,
            langbar_mgr: None,
            commits_since_flush: 0,
            shift_alone: false,
            last_context: None,
        }
    }

    /// 懒加载引擎：宿主进程第一次激活输入法时才读 3.4 MB 词库。
    fn ensure_engine(&mut self) -> Result<(), String> {
        if self.engine.is_some() {
            return Ok(());
        }
        let config_dir = config::app_data_dir();
        let data_dir = config::data_dir();
        let cfg = Config::load_or_create(&config_dir);
        let dict_path = data_dir.join("dict.txt");
        let dict = Dict::load(&dict_path)
            .map_err(|e| format!("无法加载词库 {}: {e}", dict_path.display()))?;
        let glossary = Glossary::from_dir(&data_dir);
        let userdb = UserDb::open(&config_dir.join("userdb.json"));
        self.engine = Some(Engine::with_userdb(dict, glossary, cfg, userdb));
        Ok(())
    }
}

thread_local! {
    /// 候选窗按线程持有：TSF 的按键回调永远在宿主 UI 线程上。
    static WINDOW: RefCell<Option<CandidateWindow>> = const { RefCell::new(None) };
    /// 同一线程里的所有 TIP 实例共享同一份引擎状态（TSF 可能对同一线程
    /// 多次 Activate，各建各的引擎会让按键与候选窗对不上）。
    static SHARED_STATE: std::cell::OnceCell<Arc<Mutex<State>>> = const { std::cell::OnceCell::new() };
}

fn shared_state() -> Arc<Mutex<State>> {
    SHARED_STATE.with(|cell| cell.get_or_init(|| Arc::new(Mutex::new(State::new()))).clone())
}

fn with_window<R>(f: impl FnOnce(&CandidateWindow) -> R) -> Option<R> {
    // `try_with`：进程退出阶段 TLS 可能已析构，此时静默放弃而不是 panic。
    WINDOW
        .try_with(|slot| {
            let Ok(mut guard) = slot.try_borrow_mut() else {
                return None;
            };
            if guard.is_none() {
                *guard = CandidateWindow::create().ok();
            }
            guard.as_ref().map(f)
        })
        .ok()
        .flatten()
}

#[implement(ITfTextInputProcessorEx, ITfTextInputProcessor)]
pub struct TextService {
    state: Arc<Mutex<State>>,
    sink: Mutex<Option<ITfKeyEventSink>>,
}

impl TextService {
    pub fn new() -> Self {
        crate::dll_add_ref();
        Self {
            state: shared_state(),
            sink: Mutex::new(None),
        }
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        crate::dll_release();
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> WinResult<()> {
        activate(self, ptim, tid)
    }

    fn Deactivate(&self) -> WinResult<()> {
        deactivate(self)
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32, _dwflags: u32) -> WinResult<()> {
        activate(self, ptim, tid)
    }
}

fn activate(service: &TextService_Impl, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> WinResult<()> {
    let Some(thread_mgr) = ptim.cloned() else {
        return Err(E_FAIL.into());
    };
    let inner = service.get_impl();

    // 宿主进程的 current_exe() 是 notepad.exe：数据目录必须相对本 DLL。
    // 安装布局是 <InstallDir>\bin-<hash>\mcte_tsf.dll + <InstallDir>\data\，
    // 所以同级找不到时再往上一级找。
    if let Some(dir) = std::path::Path::new(&crate::dll_path()).parent() {
        let mut resolved = None;
        for candidate in [dir.join("data"), dir.parent().map(|p| p.join("data")).unwrap_or_default()] {
            if candidate.is_dir() {
                resolved = Some(candidate);
                break;
            }
        }
        match resolved {
            Some(path) => config::set_data_dir(path),
            None => log("tsf: data directory not found next to DLL"),
        }
    }

    {
        let Ok(mut state) = inner.state.lock() else {
            return Err(E_FAIL.into());
        };
        if state.client_id == tid && state.engine.is_some() {
            // 已激活过（TSF 会重复调用），无需再次 advise。
            log(&format!("tsf activate: already active (tid={tid})"));
            return Ok(());
        }
        state.thread_mgr = Some(thread_mgr.clone());
        state.client_id = tid;
        if let Err(err) = state.ensure_engine() {
            log(&format!("tsf activate: engine unavailable: {err}"));
            return Err(E_FAIL.into());
        }
        let (dict_rows, user_words, spellings, bigrams) =
            state.engine.as_ref().map(|e| e.stats()).unwrap_or_default();
        log(&format!(
            "tsf activate: host={} pid={} tid={tid}, {dict_rows} dict rows, {user_words} user words, \
             {spellings} spellings, {bigrams} bigrams",
            host_name(),
            std::process::id()
        ));
    }

    // 鼠标点选候选：用最近一次的上下文插字
    let click_state = inner.state.clone();
    mcte_core::window::set_click_handler(move |row_index| {
        let prepared = {
            let Ok(mut state) = click_state.lock() else {
                return;
            };
            let client_id = state.client_id;
            let context = state.last_context.clone();
            let text = state
                .engine
                .as_mut()
                .and_then(|engine| engine.commit_row(row_index));
            match (text, context) {
                (Some(text), Some(context)) => Some((text, context, client_id)),
                _ => None,
            }
        };
        if let Some((text, context, client_id)) = prepared {
            log(&format!("tsf click commit: {text:?}"));
            commit_text(Some(&context), client_id, &text, &FALLBACK_USED);
        }
    });

    // 右上角「设置」：选择额外跟随的一种语言（英文固定跟随），写回 config.json
    let settings_state = inner.state.clone();
    mcte_core::window::set_settings_handler(move || {
        let current = settings_state
            .lock()
            .ok()
            .and_then(|state| state.engine.as_ref().and_then(|e| e.config().extra_language.clone()));
        let anchor = with_window(|window| window.anchor()).unwrap_or(windows::Win32::Foundation::POINT {
            x: 400,
            y: 300,
        });
        let dir = mcte_core::config::app_data_dir();
        let pick_state = settings_state.clone();
        mcte_core::settings::open(anchor, current, move |picked| {
            let mut cfg = mcte_core::config::Config::load_or_create(&dir);
            cfg.set_extra_language(picked.as_deref());
            if let Err(err) = cfg.save(&dir) {
                log(&format!("tsf settings: save failed: {err}"));
            }
            if let Ok(mut state) = pick_state.lock() {
                if let Some(engine) = state.engine.as_mut() {
                    engine.set_config(cfg);
                    refresh_window(engine);
                }
            }
        });
    });

    // 候选窗上的鼠标滚轮 = 翻页
    let wheel_state = inner.state.clone();
    mcte_core::window::set_wheel_handler(move |notches| {
        if let Ok(mut state) = wheel_state.lock() {
            if let Some(engine) = state.engine.as_mut() {
                if engine.page_count() > 1 {
                    let key = if notches > 0 {
                        mcte_core::engine::Key::PageNext
                    } else {
                        mcte_core::engine::Key::PagePrev
                    };
                    engine.handle(key);
                    refresh_window(engine);
                }
            }
        }
    });

    let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
    let sink: ITfKeyEventSink = KeySink::new(inner.state.clone()).into();
    unsafe {
        keystroke_mgr.AdviseKeyEventSink(tid, &sink, true)?;
    }
    if let Ok(mut guard) = inner.sink.lock() {
        *guard = Some(sink);
    }

    // 注册「保留键」：Ctrl+Alt+P 切中/英，与单按 Shift 切换互相补充。
    let preserved = reserved_key();
    let description: Vec<u16> = "切换中文/英文".encode_utf16().collect();
    unsafe {
        match keystroke_mgr.PreserveKey(tid, &GUID_TOGGLE, &preserved, &description) {
            Ok(()) => log("tsf: preserved Ctrl+Alt+P registered"),
            Err(err) => log(&format!("tsf: PreserveKey failed: {err}")),
        }
    }
    // 任务栏输入指示器的图标：注册一个语言栏按钮，指向 mcte.ico
    attach_langbar(&thread_mgr, inner);
    Ok(())
}

/// 注册语言栏按钮（`GUID_LBI_INPUTMODE`），并把点击行为接到「切换中/英」。
fn attach_langbar(thread_mgr: &ITfThreadMgr, inner: &TextService) {
    let Ok(mgr) = thread_mgr.cast::<ITfLangBarItemMgr>() else {
        log("tsf: ITfLangBarItemMgr unavailable, taskbar icon stays generic");
        return;
    };
    let icon_path = std::path::Path::new(&crate::dll_path())
        .parent()
        .map(|dir| {
            let beside = dir.join("mcte.ico");
            if beside.is_file() {
                beside
            } else {
                dir.parent().map(|p| p.join("mcte.ico")).unwrap_or(beside)
            }
        })
        .filter(|path| path.is_file());

    let state_for_click = inner.state.clone();
    let button = crate::langbar::LangBarButton::new(
        icon_path.as_deref(),
        "MC-DualType（左键切换中/英）".to_string(),
        move || {
            let mut toggled = None;
            if let Ok(mut state) = state_for_click.lock() {
                if let Some(engine) = state.engine.as_mut() {
                    engine.toggle_ascii_mode();
                    toggled = Some(engine.ascii_mode());
                }
            }
            if let Some(ascii) = toggled {
                log(&format!(
                    "tsf: langbar click -> {}",
                    if ascii { "ABC" } else { "中文" }
                ));
            }
        },
    );
    let item: ITfLangBarItemButton = button.into();

    match unsafe { mgr.AddItem(&item) } {
        Ok(()) => {
            log(&format!(
                "tsf: langbar item added (icon={:?})",
                icon_path.as_ref().map(|p| p.display().to_string())
            ));
            if let Ok(mut state) = inner.state.lock() {
                state.langbar = Some(item);
                state.langbar_mgr = Some(mgr);
            }
        }
        Err(err) => log(&format!("tsf: AddItem(langbar) failed: {err}")),
    }
}

fn deactivate(service: &TextService_Impl) -> WinResult<()> {
    let inner = service.get_impl();
    let Ok(mut state) = inner.state.lock() else {
        return Err(E_FAIL.into());
    };
    let client_id = state.client_id;
    if let Some(thread_mgr) = state.thread_mgr.clone() {
        if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
            let preserved = reserved_key();
            unsafe {
                let _ = keystroke_mgr.UnpreserveKey(&GUID_TOGGLE, &preserved);
                let _ = keystroke_mgr.UnadviseKeyEventSink(client_id);
            }
        }
    }
    if let Some(engine) = state.engine.as_mut() {
        if let Err(err) = engine.flush_userdb() {
            log(&format!("tsf flush userdb failed: {err}"));
        }
    }
    // 摘掉语言栏按钮，别让任务栏留着失效图标
    if let (Some(item), Some(mgr)) = (state.langbar.take(), state.langbar_mgr.take()) {
        unsafe {
            let _ = mgr.RemoveItem(&item);
        }
    }
    state.thread_mgr = None;
    state.client_id = 0;
    drop(state);

    if let Ok(mut guard) = inner.sink.lock() {
        *guard = None;
    }
    with_window(|window| window.hide());
    log("tsf deactivate");
    Ok(())
}

#[implement(ITfKeyEventSink)]
pub struct KeySink {
    state: Arc<Mutex<State>>,
}

impl KeySink {
    fn new(state: Arc<Mutex<State>>) -> Self {
        crate::dll_add_ref();
        Self { state }
    }
}

impl Drop for KeySink {
    fn drop(&mut self) {
        crate::dll_release();
    }
}

impl ITfKeyEventSink_Impl for KeySink_Impl {
    fn OnSetFocus(&self, fforeground: BOOL) -> WinResult<()> {
        if !fforeground.as_bool() {
            // 失焦：丢弃半截拼音，别把候选窗留在屏幕上。
            if let Ok(mut state) = self.get_impl().state.lock() {
                if let Some(engine) = state.engine.as_mut() {
                    engine.abort();
                }
            }
            with_window(|window| window.hide());
        }
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        let vk = wparam.0 as u32;
        if vk == VK_SHIFT_CODE {
            return Ok(BOOL::from(!ctrl_alt_win_held()));
        }
        if modifiers_held() || keymap::is_modifier(vk) {
            return Ok(FALSE);
        }
        let Some(key) = keymap::map_vk(vk) else {
            with_window(|w| {
                if w.is_showing_tip() {
                    w.hide();
                }
            });
            return Ok(FALSE);
        };
        let eager = match self.get_impl().state.lock() {
            Ok(state) => state
                .engine
                .as_ref()
                .map(|engine| engine.would_handle(key))
                .unwrap_or(false),
            Err(_) => false,
        };
        if !eager {
            with_window(|w| {
                if w.is_showing_tip() {
                    w.hide();
                }
            });
        }
        Ok(BOOL::from(eager))
    }

    fn OnKeyDown(
        &self,
        pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        let vk = wparam.0 as u32;
        if VK_SHIFT_CODE == vk {
            if let Ok(mut state) = self.get_impl().state.lock() {
                state.shift_alone = !ctrl_alt_win_held();
            }
            log("tsf: shift keydown seen");
            return Ok(FALSE);
        }
        if let Ok(mut state) = self.get_impl().state.lock() {
            state.shift_alone = false;
        }
        if !keymap::is_paging_key(vk) && (modifiers_held() || keymap::is_modifier(vk)) {
            return Ok(FALSE);
        }
        let Some(key) = keymap::map_vk(vk) else {
            return Ok(FALSE);
        };

        let mut commit: Option<String> = None;
        let mut eaten = false;
        let client_id;

        {
            let Ok(mut state) = self.get_impl().state.lock() else {
                return Ok(FALSE);
            };
            client_id = state.client_id;
            state.last_context = pic.cloned();
            {
                let Some(engine) = state.engine.as_mut() else {
                    return Ok(FALSE);
                };
                match engine.handle(key) {
                    Outcome::Ignore => {
                        if matches!(
                            key,
                            Key::Backspace | Key::Left | Key::Right | Key::Up | Key::Down
                        ) {
                            engine.clear_recent_commits();
                        }
                    }
                    Outcome::Consumed => eaten = true,
                    Outcome::Commit(text) => {
                        eaten = true;
                        commit = Some(text);
                    }
                }
                if eaten {
                    refresh_window(engine);
                }
            }
            if commit.is_some() {
                state.commits_since_flush += 1;
                if state.commits_since_flush >= FLUSH_EVERY_COMMITS {
                    state.commits_since_flush = 0;
                    if let Some(engine) = state.engine.as_mut() {
                        if let Err(err) = engine.flush_userdb() {
                            log(&format!("tsf flush userdb failed: {err}"));
                        }
                    }
                }
            }
        }

        if let Some(text) = commit {
            let context = pic.cloned();
            log(&format!("tsf commit: {text:?}"));
            commit_text(context.as_ref(), client_id, &text, &FALLBACK_USED);
        }

        Ok(BOOL::from(eaten))
    }

    fn OnTestKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        // 单独按下的 Shift 抬起时要自己处理（切中/英），所以这里必须认领。
        if wparam.0 as u32 == VK_SHIFT_CODE {
            let alone = self
                .get_impl()
                .state
                .lock()
                .map(|state| state.shift_alone)
                .unwrap_or(false);
            return Ok(BOOL::from(alone));
        }
        Ok(FALSE)
    }

    fn OnKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        if wparam.0 as u32 != VK_SHIFT_CODE {
            return Ok(FALSE);
        }
        log("tsf: shift keyup seen");
        let mut toggled = None;
        if let Ok(mut state) = self.get_impl().state.lock() {
            if state.shift_alone {
                state.shift_alone = false;
                if let Some(engine) = state.engine.as_mut() {
                    engine.toggle_ascii_mode();
                    toggled = Some(engine.ascii_mode());
                }
            }
        }
        match toggled {
            Some(ascii) => {
                let text = if ascii { "英语模式（中文跟随）" } else { "中文模式（多语跟随）" };
                with_window(|window| {
                    window.flash_tip("Shift", text, 700);
                });
                log(&format!("tsf: shift toggle -> {}", if ascii { "英语" } else { "中文" }));
                Ok(windows::core::BOOL::from(true))
            }
            None => Ok(FALSE),
        }
    }

    fn OnPreservedKey(
        &self,
        _pic: Ref<'_, ITfContext>,
        rguid: *const windows::core::GUID,
    ) -> WinResult<BOOL> {
        if rguid.is_null() {
            log("tsf: OnPreservedKey(null)");
            return Ok(FALSE);
        }
        let pressed = unsafe { *rguid };
        if pressed != GUID_TOGGLE {
            log("tsf: OnPreservedKey(other guid)");
            return Ok(FALSE);
        }
        log("tsf: OnPreservedKey(ours)");
        toggle_mode(self);
        Ok(windows::core::BOOL::from(true))
    }
}

/// 保留键定义：Ctrl+Alt+P
fn reserved_key() -> TF_PRESERVEDKEY {
    TF_PRESERVEDKEY {
        uVKey: VK_P_CODE,
        uModifiers: TF_MOD_CONTROL | TF_MOD_ALT,
    }
}

/// 切换中/英，并在候选窗里闪一条提示（下一次按键时会被替换掉）。
fn toggle_mode(sink: &KeySink_Impl) {
    let mut result = None;
    if let Ok(mut state) = sink.get_impl().state.lock() {
        if let Some(engine) = state.engine.as_mut() {
            engine.toggle_ascii_mode();
            result = Some(engine.ascii_mode());
        }
    }
    if let Some(ascii) = result {
        let text = if ascii { "英语模式（中文跟随）" } else { "中文模式（多语跟随）" };
        with_window(|window| {
            window.flash_tip("Ctrl+Alt+P", text, 700);
        });
        log(&format!("tsf: toggle -> {}", if ascii { "英语" } else { "中文" }));
    }
}

fn refresh_window(engine: &Engine) {
    let composing = engine.is_composing();
    let rows = engine.rows().len();
    let preedit = engine.preedit();
    let showed = with_window(|window| {
        if composing {
            window.update(&preedit, engine.rows(), engine.highlight(), engine.page(), engine.page_count());
            true
        } else {
            window.hide();
            false
        }
    });
    match showed {
        None => log("tsf: candidate window unavailable"),
        // 每个按键都写日志太吵；只在有候选时记一行「首帧」，便于排障。
        Some(true) if rows > 0 && preedit.len() <= 6 && !preedit.contains(' ') => {
            log(&format!("tsf: preedit={preedit:?}, {rows} rows"));
        }
        _ => {}
    }
}

/// 把文本插进当前选区。
///
/// 返回 `false` 表示宿主不支持 TIP 插入（终端、部分 Electron 宿主的 ACP 文本存储
/// 就是这样），调用方应退回 `SendInput` 键盘注入 —— 那条路不依赖宿主实现。
fn insert_text(context: &ITfContext, client_id: u32, text: &str) -> bool {
    let session: ITfEditSession = InsertSession {
        context: context.clone(),
        text: text.to_string(),
    }
    .into();
    unsafe {
        match context.RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE) {
            Ok(hr) => hr.is_ok(),
            Err(err) => {
                log(&format!("tsf insert failed: {err}"));
                false
            }
        }
    }
}

/// 统一的上屏入口：先试 TIP 插入，不行就键盘注入。
fn commit_text(context: Option<&ITfContext>, client_id: u32, text: &str, fallback_once: &AtomicBool) {
    let mut inserted = false;
    if let Some(ctx) = context {
        inserted = insert_text(ctx, client_id, text);
    }
    if inserted {
        return;
    }
    // 宿主不支持 → SendInput（终端里唯一可靠的方式）
    if !fallback_once.swap(true, Ordering::SeqCst) {
        log("tsf: 宿主不支持 TIP 插入，改用 SendInput 注入（终端 / Electron 常见）");
    }
    mcte_core::inject::type_text(text);
}

#[implement(ITfEditSession)]
struct InsertSession {
    context: ITfContext,
    text: String,
}

impl ITfEditSession_Impl for InsertSession_Impl {
    fn DoEditSession(&self, ec: u32) -> WinResult<()> {
        let inner = self.get_impl();
        let insert: ITfInsertAtSelection = inner.context.cast()?;
        let units: Vec<u16> = inner.text.encode_utf16().collect();
        unsafe {
            // 0 = 插入文本（不需要额外标志位）
            insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &units)?;
        }
        Ok(())
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

/// 当前宿主进程名（日志诊断用：一眼看出是终端、记事本还是浏览器）。
fn host_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "?".to_string())
}

fn modifiers_held() -> bool {
    unsafe {
        let down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
            (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
        };
        down(VK_CONTROL) || down(VK_MENU) || down(VK_SHIFT) || down(VK_LWIN) || down(VK_RWIN)
    }
}
