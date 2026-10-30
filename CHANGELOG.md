# Changelog

> **MC-DualType**: A bidirectional smart input method that makes learning English effortless, and empowers non-native speakers to master and type Chinese with zero learning curve.

All notable changes across all versions are documented in this file, along with direct download links for release packages.

---

## [v1.3.7] - 2026-10-30

### 📦 Downloads for v1.3.7
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.7.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.7/MC-DualType-1.3.7.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.7.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.7/MC-DualType-Setup-1.3.7.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.7-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.7/MC-DualType-1.3.7-Portable.zip)

### AI Workbench Full-Length Scrollable Rendering, History Drawer, Stop Task & Zero-Contamination Chat Bridge
- **Flicker-Free Double Buffering & Smooth Mouse Wheel Scrolling**:
  - **100% Flicker-Free Double Buffering**: Implemented offscreen double buffering (`CreateCompatibleDC` + `CreateCompatibleBitmap` + `BitBlt`) across the entire workbench window. Eliminates all screen tearing and strobing during mouse wheel scrolling.
  - **Eliminated 4-Line Truncation**: Replaced hardcoded line caps with `draw_wrapped_text_scrollable`. DeepSeek in-depth summaries, financial bill breakdowns, tables, and long translations now render in their entirety without cutoffs.
  - **Clean Single Copy Button Layout**: Removed duplicate stacked copy button. Preserves a single, high-contrast `[ 📋 复制 ]` badge on the divider line and clean `[ 💬 引用 ]` badge for original text.
- **Dedicated Task Stop Button & Immediate Generation Cancellation (`🛑 停止任务`)**:
  - When an AI request is in flight, the primary action button transforms into a prominent coral red `🛑 停止任务` button.
  - Clicking `🛑 停止任务` or pressing `Esc` immediately cancels the background thread, dispatches stop generation commands (`stop_generation_js` / Escape) to DeepSeek/Doubao web tabs, and restores UI readiness.
- **Third-Party Input Bar Zero-Contamination Clearing (`clear_input_js`)**:
  - Fixed an issue where prompt text entered via CDP remained stuck in DeepSeek or Doubao web input boxes, repeatedly contaminating subsequent chat interactions.
  - Implemented multi-stage clearing: proactively clears residual drafts before typing, triggers native send buttons with hardware Enter fallbacks, and executes `clear_input_js()` 350ms post-send and upon response completion to guarantee pristine third-party chat inputs.
- **Integrated Left Conversation History Sidebar (`📜 对话历史`)**:
  - Added a collapsible, elegant left history drawer displaying past conversation pairs aggregated from `chat.jsonl`.
  - Includes a `[ ➕ 新建对话 ]` action button to clear current cards and launch a fresh session in one click.
  - Clicking any history item instantly restores the prompt, AI response, and backend badge into the workbench with full scrollability, copying, and quoting capabilities.
  - Includes a `[ ◀ ]` collapse button and `[ 📜 历史 ]` toggle to expand or collapse the sidebar on demand.
- **Original Text Quote & Local File Path Extraction Bridge**:
  - Clicking `[ 💬 引用 ]` immediately quotes selected text into the input bar with auto-focus.
  - Automatically detects local file paths (`C:\...`) pasted into the input bar, extracts text locally via native Windows parsers (PDF, DOCX, TXT), and streams content to DeepSeek for analysis.

---

## [v1.3.6] - 2026-10-30

### 📦 Downloads for v1.3.6
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.6.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.6/MC-DualType-1.3.6.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.6.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.6/MC-DualType-Setup-1.3.6.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.6-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.6/MC-DualType-1.3.6-Portable.zip)

### Installer Language Edition Selection & Raw English Direct Passthrough (Shift & Ctrl+Space Zero Interference)
- **Installer Language Edition Selection & Windows System UI Auto-Detection**:
  - **Language Edition Architecture**: Officially split the input experience into two tailored editions:
    - **🇨🇳 Chinese Edition (中文版 - Default on Chinese Windows)**: Designed for Chinese speakers learning English and typing bilingual text. Default mode is Pinyin input (showing Chinese candidate cards with English and multilingual translations). Tapping Shift or Ctrl+Space toggles to raw native English direct input.
    - **🇺🇸 English Edition (英文版 - Default on English Windows)**: Designed for English native speakers learning Chinese. Default mode is smart English input (typing English words produces candidate cards with Chinese translations). Tapping Shift or Ctrl+Space toggles to raw native English direct input.
  - **Automatic Windows Locale Detection**: Automatically queries Windows user UI language (`GetUserDefaultUILanguage()`); defaults to Chinese edition on Chinese Windows (`zh-CN`, `zh-TW`, `zh-HK`), and English edition on English/international Windows (`en-US`, etc.).
  - **Interactive Setup Prompt**: `Setup.exe` prompts the user upon installation to choose between Chinese and English editions with clear system recommendations, and supports silent `-Edition Chinese|English` deployment.
- **True Raw English Direct Passthrough (Eliminated Keyboard & Mouse Typing Interference)**:
  - **Identified Root Cause**: In previous versions, switching to "English mode" only toggled `ascii_mode`, but the engine still intercepted keystrokes, consumed Space and Enter, and popped up candidate cards. This made normal English typing, programming, gaming, and mouse interactions severely interfered with.
  - **100% Native Passthrough**: Implemented `direct_mode` in both the hook engine and TSF text service. In raw English direct mode, all keystrokes (`a-z`, `0-9`, symbols, Space, Enter, Backspace, Esc, function keys) completely bypass the IME and pass directly to the target application (`hook::pass_through` / TSF returns `FALSE`).
  - **Zero Visual & Interaction Overhead**: In raw direct mode, candidate windows remain completely hidden, no keystrokes are intercepted, and mouse interactions operate completely undisturbed, behaving identically to the native Windows US English keyboard.
- **Dual Hotkey Toggle (Shift & Ctrl+Space)**:
  - Supported both short single-tap on `Shift` and classic Windows `Ctrl + Space` to toggle between the active smart bilingual mode and raw English direct passthrough.
  - Displays instant subtle HUD indicator: `🔤 英文（原始直接输入）` / `🇨🇳 中文模式（多语跟随）` / `🇺🇸 英文模式（中文跟随）`.
- **Anytime Edition Switching**:
  - Added 1-click edition switching in the system tray menu (`切换为：英文版` / `切换为：中文版`).
  - Added dedicated Language Edition radio buttons in Settings (`设置 -> 🌐 跟随语言设置 -> 语言版本`).

---

## [v1.3.5] - 2026-10-30

### 📦 Downloads for v1.3.5
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.5.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.5/MC-DualType-1.3.5.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.5.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.5/MC-DualType-Setup-1.3.5.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.5-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.5/MC-DualType-1.3.5-Portable.zip)

### Agent Skills Architecture, Native SAPI Speech & Dual-Channel High-Fidelity Rich Formatting
- **Agent Skills Architecture in AI Workbench**:
  - **Dynamic Agent Skill Bar**: Integrated interactive skill pills into the workbench top header: `[ 🌐 翻译 ]` (Translate), `[ 📝 总结 ]` (Summarize), `[ 💡 解释 ]` (Explain), `[ 💬 对话 ]` (Chat), and `[ 🔊 朗读 ]` (Speech).
  - **Task-Tailored Prompt Engine**: Decoupled AI invocation into structured skills. Each skill automatically crafts optimal prompt engineering directives adapted to DeepSeek, Doubao, or cloud API models.
  - **Context Re-execution on Skill Switch**: Clicking any skill immediately switches the active mode and intelligently re-executes the new task on either the input text or the loaded document without retyping.
  - **Dynamic Action Button Adaptation**: The primary action button at the bottom right dynamically updates its label according to the active skill (`发送翻译`, `生成总结`, `深度解释`, `发送对话`), providing clear visual clarity.
- **Native Windows SAPI Offline Speech Synthesis (`[ 🔊 朗读 ]` / `[ ⏹ 停止 ]`)**:
  - **Zero-Dependency Offline TTS**: Built-in Windows SAPI `SpVoice` speech synthesis engine in `core/src/speech.rs`. Requires zero network connection, zero third-party token, and zero API keys.
  - **Non-Blocking Interruptible Playback**: Executes speech asynchronously in an isolated process. Clicking `🔊 朗读` begins reading either the translation or the original text while switching the pill to green `[ ⏹ 停止 ]`; clicking again instantly interrupts and stops speech.
- **Dual-Channel High-Fidelity Rich Clipboard Copying (Markdown + CF_HTML)**:
  - **Resolved Flat Text Stripping**: Fixed the issue where copying AI output stripped formatting into unstyled text. Previously, only `CF_UNICODETEXT` was written to the clipboard.
  - **Windows `CF_HTML` Packet Generator**: Created `core/src/format.rs` with a custom Markdown-to-HTML converter and byte-accurate Windows Clipboard `HTML Format` header generator with `StartHTML`, `EndHTML`, `StartFragment`, and `EndFragment` offset calculations.
  - **Dual-Format Clipboard Dispatch**: In `core/src/window.rs`, `copy_to_clipboard` registers both `CF_UNICODETEXT` (authentic Markdown) and `HTML Format` (styled HTML). Pasting into Microsoft Word, WeChat, Feishu, Outlook, Notion, or browsers preserves headers, bold text, code blocks, and bullet lists; pasting into code editors (VS Code, Notepad) outputs clean Markdown.
  - **Authentic Markdown Extraction**: Upgraded `deepseek_last_reply_js()` and `doubao_last_reply_js()` to extract authentic Markdown syntax directly from React Fiber `memoizedProps` before falling back to `innerText`.
- **Card Typography & Visual Structure Optimization**:
  - **Paragraph Breathing Room**: Enhanced `draw_wrapped_text` to preserve empty lines (`\n\n`) as half-line height spacers (`line_h / 2`), eliminating dense blocks of text.
  - **Bullet List Indentation**: Indents unordered list items (`- `, `* `, `• `) by 14px for structured visual hierarchy.
  - **Hand Cursor Hit-Testing**: Registered `IDC_HAND` cursor feedback across all skill pills, language selectors, speech pills, and action buttons.

---

## [v1.3.4] - 2026-10-30

### 📦 Downloads for v1.3.4
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.4.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.4/MC-DualType-1.3.4.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.4.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.4/MC-DualType-Setup-1.3.4.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.4-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.4/MC-DualType-1.3.4-Portable.zip)

### AI Workbench Input Send Fix (RefCell Re-entrancy) & File Upload Non-Blocking DeepSeek Pipeline
- **AI Workbench Input Send & Enter Key Fix (Win32 RefCell Re-entrancy Resolution)**:
  - **Identified Root Cause**: In `workbench.rs`, `get_input_text()` held a mutable borrow of `RefCell<Option<State>>` via `with_state` while dispatching `SendMessageW(st.input, WM_GETTEXTLENGTH)`. Because `st.input` was subclassed with `input_proc`, Windows synchronously called `input_proc` on the exact same thread. `input_proc` then attempted to acquire `with_state(|st| st.input_prev)` to forward messages to the original edit procedure. Due to `RefCell` re-entrancy rules, `try_borrow_mut()` failed, causing `prev` to return `0` and fallback to `DefWindowProcW`, which does not manage edit control buffers and returned 0 length. Consequently, `get_input_text()` always returned `""` (empty string) and early-exited on Enter or Send click!
  - **Fix**: Replaced `st.input_prev` with a thread-safe `static INPUT_PREV_PROC: AtomicIsize`, decoupling window message processing from internal state locks entirely. Refactored `get_input_text()` and `clear_input_text()` so `with_state` only extracts the `HWND` and immediately releases all borrows before calling Win32 `SendMessageW`. Enter key presses and "发送翻译" button clicks now capture text and dispatch translations instantly.
  - **Input Focus & Activation**: Added click hit-testing for the input bar and automated `SetFocus(st.input)` upon closing the file picker dialog, ensuring immediate keyboard input readiness.
- **File Upload Stuck in "Processing" & Never Replying Fix**:
  - **Identified Root Cause 1 (JSON Escaping Bottleneck)**: When uploading documents via `+ 上传文件`, `translate_text_via_desktop` forced large structured files (e.g., Markdown files with tables, headers, and code snippets) into a strict multi-key JSON schema. DeepSeek-R1 (with deep thinking enabled) struggled to escape newlines, quotes, and symbols into JSON strings, spending 90+ seconds in reasoning and triggering timeout while leaving the UI stuck in "⏳ 处理中...".
  - **Identified Root Cause 2 (Deep Thinking Stream Confusion)**: In `webchat.rs`, `deepseek_last_reply_js()` selected all `.ds-markdown` elements. DeepSeek renders ongoing reasoning tokens inside `.ds-think-content .ds-markdown`, causing `wait_for_reply` to confuse the active thinking stream with the final answer.
  - **Fix**:
    - Streamlined the translation prompt to request direct clean markdown translation without restrictive JSON wrappers.
    - Updated `deepseek_last_reply_js()` and `deepseek_latest_chat()` to filter out `.ds-think-content`, cleanly isolating the genuine final translation.
    - Enhanced `deepseek_set_input_js()` and `deepseek_send_js()` with reactive prototype setters and input event triggers, preventing button disabling issues.
    - Document translations now complete in ~3-5 seconds and update the translation pane smoothly.

---

## [v1.3.3] - 2026-10-30

### 📦 Downloads for v1.3.3
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.3.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.3/MC-DualType-1.3.3.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.3.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.3/MC-DualType-Setup-1.3.3.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.3-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.3/MC-DualType-1.3.3-Portable.zip)

### Interactive Target Language Selector, Win32 Input Send Fix & Multi-Encoding File Parser
- **Interactive Target Language Selector (`[EN ▾]`) & Dynamic Prompt Injection**:
  - Transformed the static target language badge into a fully interactive Win32 popup menu (`[EN ▾]`). Users can click the language pill to switch target translation languages on the fly: English (EN), Chinese (ZH), Japanese (JA), German (DE), French (FR), Spanish (ES), Korean (KO), and Russian (RU).
  - Dynamically synthesizes the AI translation prompt according to the selected target language (`请将以下内容准确翻译成【{target_display}】（语言代码: {target_lang}），只输出译文，格式保持整洁：\n{text}`), ensuring direct, high-precision translations without unnecessary conversational fluff.
- **Win32 Input Retrieval & Send Dispatch Fix**:
  - **Identified Root Cause**: In subclassed Win32 child EDIT controls, `GetWindowTextLengthW` returned 0, causing `send_current_input()` to treat typed text as empty and exit early.
  - **Fix**: Replaced text retrieval with direct message dispatches: `SendMessageW(hwnd, WM_GETTEXTLENGTH)` and `SendMessageW(hwnd, WM_GETTEXT)`. Input clearing now dispatches `SendMessageW(hwnd, WM_SETTEXT)`. Enter key and the "发送翻译" button now fire immediately and synchronously.
- **Universal Multi-Encoding Document Parser (GBK / ANSI / UTF-8 / GB18030)**:
  - **Identified Root Cause**: Document files (`.txt`, `.pdf`, `.md`) created on Chinese Windows systems are often encoded in GBK / ANSI. Standard Rust `fs::read_to_string` rejected non-UTF-8 bytes with "stream did not contain valid UTF-8", triggering "打开文件失败".
  - **Fix**: Implemented adaptive multi-encoding decoding (`decode_text_bytes`) testing UTF-8 -> Code Page 936 (GBK) -> System ACP -> Lossy fallback. Non-UTF-8 text files and documents now open and parse with zero corruption or failure.
- **Dynamic File Translation Display**:
  - Uploading a file via `+ 上传文件` immediately populates the "原文" (Original Text) pane with the filename and document preview, dispatches the content to DeepSeek or Doubao, and streams the translated output into the "译文" (Translation) pane in real time.

---

## [v1.3.2] - 2026-10-30

### 📦 Downloads for v1.3.2
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.2.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.2/MC-DualType-1.3.2.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-Setup-1.3.2.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.2/MC-DualType-Setup-1.3.2.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.2-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.2/MC-DualType-1.3.2-Portable.zip)

### DeepSeek Automated Send & AI File Translation Pipeline Overhaul
- **DeepSeek Automated Message Dispatch & Send Automation**:
  - **Identified Root Cause**: In modern DeepSeek web frontends, dispatching synthetic `KeyboardEvent` (Enter) on textareas is ignored by the reactive framework. Without clicking the circular blue send button (`.ds-button--circle`), messages remained stuck in the input box indefinitely.
  - **Full Automation**: Implemented automated circular send button detection and dispatch in `deepseek_send_js()`, synchronized with reactive `input` and `change` events. AI query and translation responses now arrive reliably in ~1.5 seconds.
- **End-to-End File Translation Pipeline (`+ 上传文件`)**:
  - Completely revitalized the file upload workflow. Clicking `+ 上传文件` launches the system explorer dialog via an STA COM-initialized thread with CBT focus elevation.
  - Full support for multi-format text and document extraction (`.pdf`, `.docx`, `.xlsx`, `.txt`, `.md`, `.json`, `.csv`) as well as image OCR (`.png`, `.jpg`, `.bmp`, `.webp`).
  - Selected file name and extracted preview text are immediately rendered onto the workbench card, dispatched to the active AI backend (DeepSeek / Doubao), and the complete bilingual translation is returned and updated directly on screen.
- **Live AI Workspace Dynamic Sync**:
  - Removed outdated static placeholder articles. When opening the AI Workbench, it dynamically pulls the latest authentic conversation from the connected AI backend (DeepSeek or Doubao), reflecting the actual conversational context.
- **Mouse Cursor & Focus Normalization**:
  - Fixed missing arrow cursor (`IDC_ARROW`) in window class definition.
  - Added responsive hand pointer (`IDC_HAND`) over all interactive buttons and copy controls, while maintaining native I-beam cursor (`IDC_IBEAM`) inside the conversation edit field.
- **Clean Aesthetic & High-Legibility UI**:
  - Replaced dashed dividers with clean, subtle solid separation lines.
  - Enlarged bottom toolbar and buttons (height 44px, width 120px) with bold high-contrast typography, ensuring maximum legibility and clickability.

---

## [v1.3.1] - 2026-10-29

### 📦 Downloads for v1.3.1
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.1.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-1.3.1-Setup.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Setup.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.1-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Portable.zip)

### Critical Engine Freeze Fixes, Particle Intelligence & Streamlined AI Workbench
- **Eliminated Severe Typing Freezes (`anxin`, `shili`, `shilalah`)**:
  - **Root Cause Identified**: When entering non-exact pinyin syllables (e.g. typing `x` after `an` or typing `shil`), the engine fell back to reverse lookup and returned hybrid entries (such as `anorexia 食欲缺乏` or `shillelagh 橡树棍`). The modal particle analyzer `particle::translate_particle_phrase()` previously executed recursive \(O(N^3)\) sub-string decompositions on strings containing spaces and English letters, locking the UI thread for up to 27 seconds.
  - **Zero-Latency Barrier**: Restricted particle/aspect morphological analysis exclusively to pure Chinese phrases of $\le 6$ characters. 
  - **Keystroke Performance**: Continuous typing latency reduced from **26.91s to 0.40ms** (over 67,000x faster, ensuring 100% stutter-free continuous typing).
- **Intelligent Dynamic Learning & Missing Syllable Auto-Adaptation (`kanma` -> `看吗`)**:
  - The offline dictionary does not maintain `kanma` as a single compound syllable; the engine's Viterbi dynamic segmentation automatically splits it into `kan` (看) and `ma` (吗).
  - Built-in self-learning engine (`UserDb`) autonomously memorized `kanma` -> `看吗` into persistent local memory (`userdb.json`) upon user confirmation, ranking it to top position on subsequent keystrokes without requiring dictionary rebuilds.
- **AI Workbench UI & Interaction Overhaul**:
  - **Removed Distracting Dashed Borders**: Completely eliminated legacy dashed bounding boxes (`draw_dashed_rect`), presenting a clean, modern card aesthetic.
  - **Enlarged High-Contrast Controls**: Toolbar buttons upgraded to prominent 42px height with bold 15-16px typography (`[+ 上传文件]` and `[发送翻译]`).
  - **Interactive Hand Pointer (`IDC_HAND`)**: Hand cursor feedback now immediately displays over all interactive elements (buttons, copy icon `📋`, and chevrons).
  - **Enter Key Instant Transmission**: Input box now captures `VK_RETURN` (`WM_GETDLGCODE` / `DLGC_WANTALLKEYS`) to directly trigger translation queries without needing manual clicks, silencing the default Windows ding sound.
  - **Live Doubao Chat Synchronization**: Automatically syncs active conversations directly from Doubao Desktop via local CDP bridge on launch, providing instant working feedback.

---

## [v1.3.0] - 2026-10-29

### 📦 Downloads for v1.3.0
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.0.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-1.3.0-Setup.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Setup.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.0-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Portable.zip)

### V1.3 UI Redesign & Doubao-Style Instant AI Workbench
- **Streamlined Candidate Window UI**:
  - Removed redundant sidebar buttons (`+` and `📷`) from the candidate typing box to create a focused, pure, distraction-free typing experience.
  - Symmetrically balanced card margins (`LEFT_PAD = RIGHT_PAD = 34px`) with left-aligned preedit pill (`BUBBLE_LEFT_X = 34`).
  - Refined pagination chevrons (`‹` and `›`) with proper horizontal alignment and high-contrast styling.
- **Doubao-Style Instant Desktop Workflow (Zero Reboot Required)**:
  - **No System Reboot Needed**: Fully operational immediately upon installation. Eliminated the misconception of requiring a Windows restart.
  - **Global Hotkey `Ctrl + Alt + Space`**: Instant pop-up of the AI Workbench anytime, anywhere across all Windows applications without switching input methods or pressing `Win + Space`.
  - **Smart Anti-Drift Positioning**: Intelligent cursor and window positioning algorithm that detects foreground application bounds (in Chrome, WeChat, VS Code, Edge), preventing candidate or assistant windows from drifting to secondary monitors or off-screen corners.
- **Adaptive Bilingual AI Slider & Independent Workbench**:
  - **Inner Bilingual White Card**: Elegant `#FAF9F8` outer container with inner white rounded card (`#FFFFFF`, border `#D9DED2`, radius `18px`).
  - **Status Indicator**: Header featuring `[译文]` pill, bidirectional icon `⇄`, and `[EN] ●` pill with emerald green connection status indicator light.
  - **Bilingual Comparison View**: Clear bold headlines, clean wrapped paragraph typography, middle dashed divider line, and 1-click clipboard copy button `📋` (with temporary "已复制" feedback).
  - **Adaptive Dialog Input Bar**: Bottom-left conversation input bar stretches horizontally to adapt to any window width; supports direct keyboard input, Enter to send, Shift+Enter for newline, and Esc to close.
  - **Dashed Third-Party Button Mapping Area**: Bottom-right container wrapped in `#787878` dashed border with dynamically calculated adaptive width based on active mapped buttons: `深度思考` (Deep Thinking), `联网` (Web Search), `+上传` (Upload), and `发送` (Send).
- **Standalone AI Workbench Window (`--workbench`)**:
  - Dedicated non-blocking Win32 UI window (`McteWorkbench`) accessible from CLI, system tray menu ("打开 AI 工作台"), or via global shortcut `Ctrl + Alt + Space`.
  - Full window resizing support with automatic adaptive layout re-calculation and local persistent conversation history synchronization.
- **CDP Web/Desktop Bridge & Multimodal OCR**:
  - Auto-detects running local AI clients: Doubao Desktop (CDP `:9444`) and DeepSeek Browser instance (CDP `:9333`).
  - Allows direct injection and toggle synchronization for third-party buttons (`深度思考`, `联网`) via CDP.
- **Safety, Anti-Deadlock & Stability**:
  - Strict Win32 `RefCell` re-entrancy protection eliminating crashes during synchronous message callbacks.
  - Completely hidden background process console windows (`CREATE_NO_WINDOW`).
  - Asynchronous background thread execution for all AI network and multimodal requests, ensuring smooth typing without UI stalls.

---

## [v1.1.2] - 2026-10-28

### 📦 Downloads for v1.1.2
- **Windows Standard Installer**: [MC-DualType-1.1.2.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.2/MC-DualType-1.1.2.msi)
- **Standalone Setup**: [MC-DualType-1.1.2-Setup.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.2/MC-DualType-1.1.2-Setup.exe)

### Overlays stop covering the screen, file upload actually works, window stops drifting
- **Fixed "a panel/canvas keeps covering the screen until you quit"**:
  - Once the result panel (slider) opened there was **no automatic way to close it** — only the tiny `⌄` handle collapsed it, so intermediate states like "选择文件中…" stayed on top forever.
  - Now: **Esc dismisses overlays first** (and no longer swallows the key, so modal dialogs still cancel with Esc), **typing closes the panel**, the `+`/snapshot flows reset immediately on cancel or error, and a panel stuck for more than 90 s closes itself.
  - Snapshot overlay: it never took the foreground, so Esc could not reach it (right-click was the only exit), and a failed screen grab painted a **full-screen black canvas**. It now takes focus, reports an error instead of showing black, and self-closes after 120 s.
- **Fixed "uploading a file never translates"** (two independent causes, both verified):
  1. The common file dialog (`GetOpenFileNameW` + `OFN_EXPLORER`) **requires the calling thread to have initialised OLE/COM**. It was being called from a worker thread that never initialised COM, with `owner=None`, so no dialog ever appeared and the panel stayed on "选择文件中…". It now runs on the UI thread after `CoInitializeEx(STA)`, owns to the foreground window, is forced visible with a CBT hook, and failures (including `CommDlgExtendedError`) are logged.
  2. "Option 2: auto-bridge" treated **"Chrome is running" as "a Google/Gemini endpoint is ready"** and auto-saved a **keyless public endpoint**, so every request failed with 401. Now only genuinely reachable local HTTP endpoints (Ollama / LM Studio) count as ready; cloud channels are labelled "needs an API key", and when nothing is reachable the dialog says so instead of claiming a connection.
- **Fixed the candidate window "drifting"**: with the slider open but no candidates, `measure()` injected a **hardcoded design mockup** (是吗 / 是嘛 / 时髦 … with the pinyin pinned to `shima`), so a fake candidate strip stayed on screen and the window kept re-anchoring around it. The mock data is gone; that state now renders the panel alone (880 × 452, covered by a unit test).
- **Also fixed**: the TSF build (Win+Space path) used to `hide()` the candidate window whenever there was no composition, so upload/snapshot results were **never visible**; it now shows the result panel like the hook build and supports Esc dismissal too.
- **Installer note**: the resident process started by the installer's `StartApp` action inherits administrator rights (a side effect of the UAC-elevated install). Quit it from the tray and log in again if you want a normal-integrity instance.

## [v1.1.1] - 2026-10-28

### Settings Window Crash & Off-screen Buttons Fix
- **Fixed "Settings → Option 2 → click Scan" killing the whole IME process**:
  - Root cause: the settings window kept a `RefCell` mutable borrow across the whole `WM_COMMAND` handler while still calling Win32 inside it (`SetWindowText` for the hint text, `LB_ADDSTRING` / `LB_SETCURSEL` for the bridge list, `DestroyWindow` when closing). Those calls **re-enter** the window procedure synchronously (`WM_CTLCOLORSTATIC` / `WM_CTLCOLORLISTBOX` / `WM_DRAWITEM` / `WM_DESTROY`), and the re-entrant path called `borrow()` again → `RefCell already mutably borrowed` → a panic that cannot unwind across the FFI boundary → process abort, taking the tray icon and the IME down with it.
  - State is now taken out of the cell before handling and put back afterwards, so no borrow is held across any Win32 call; paint messages read the brush, fonts and active tab from dedicated `Cell` side channels.
  - The same defect silently affected every other interaction: test-connection/model fetch, selecting a bridge row, save, cancel, the title-bar close button, and async result rendering.
- **Fixed the bottom buttons being pushed outside the visible client area**:
  - The window was created with `630` as its *client* height, but `CreateWindowExW` expects the **whole window** size, so the title bar ate 64px of it. At 175% display scaling the client area measured 536×566 while "确定保存并返回输入法" and "取消" sat at y=560..598 — effectively invisible and unclickable.
  - The window is now grown by the measured `window − client` delta so the client area really is 560×630, and the bottom buttons are anchored to the client bottom (`client height − 24 − 38`), so no DPI or caption height can push them out again.
  - The card background and the language list now size themselves from the client rect, the window is centred inside the monitor work area and clamped to it, and layout is redone on `WM_DPICHANGED`.
- **Other fixes**:
  - The first local-endpoint scan now actually runs when the dialog opens (it used to fire before the window handle was registered, so it silently did nothing until you pressed Scan).
  - Radio options 1/2 are no longer both shown as selected (a plain window has no dialog manager to enforce exclusivity).
  - Both options are laid out on their own full-width rows, so "选项一：API Key 统一调用 (DeepSeek / OpenAI)" is no longer truncated.
  - Owner-drawn button text now uses the UI font instead of falling back to the system font, and bridge status uses `●` / `○` instead of emoji that the list font cannot render.
  - The Scan button is single-flight: repeated clicks no longer pile up scan threads.

## [v1.1.0] - 2026-10-28

### AI Access, Multimodal Recognition, Self-Upgrading Closed Loop & Slider UI (V1.1 AI Upgrade)
- **Settings AI Access Options (Two Full Workflows)**:
  - **Option 1 (API Key Direct Invocation)**: Configure Base URL and API Key, dynamic connection testing via `/v1/models`, dynamic model dropdown selection, save & return to IME.
  - **Option 2 (Local Web/Desktop Client Bridge)**: One-click local bridge scanner checking local DeepSeek, Doubao, Google/Gemini proxies, Ollama, LM Studio, etc., auto-connecting whichever is ready, model selection, and saving.
- **AI Closed-Loop Optimization & Self-Upgrading Automation**:
  - Automatically injects recognized rare characters and optimized translations into local glossary and persistent user learning database (`userdb.json`).
  - Seamlessly reloads on subsequent sessions, forming a complete self-healing, self-improving offline closed loop.
- **Multimodal File Upload & Snapshot Screen Marquee**:
  - Full-text extraction and translation across both access options for any multimodal-capable model.
  - **`+` Button**: Native file picker for images (PNG, JPG, BMP, WebP) or documents (TXT, MD) to extract and translate.
  - **`快照` Button (📷)**: Full-screen crosshair marquee selection across multiple displays with darkened background, cyan border, and live dimensions badge.
- **Expandable Slider UI (Pixel-Perfect Alignment with Design Spec)**:
  - 196px frosted dark slider panel expanding smoothly above candidate bar with top chevron `⌄` handle.
  - Status capsule `译文 ⇄ EN ●` with emerald active indicator.
  - Translated headline and auto-wrapped body with fine divider line.
  - Quick-copy button `📋` for instant clipboard export.
  - Original text section with `原文` badge.

## [v1.0.5] - 2026-10-27

### Enhanced English Input Mode & Translation De-duplication
- **Eliminated Duplicate Chinese Translations in English Mode**:
  - Implemented semantic translation signature deduplication to filter out inflected word variants that share identical definitions (e.g. `checks`, `checked` sharing `check`'s definitions, and `checkmated`, `checkmates`, `checkmating` sharing `checkmate`'s definitions).
  - Frees up candidate slots for distinct associative words with unique meanings (`checkout`, `checklist`, `check-in`, `checkpoint`, `checking`, etc.).
  - Deduplicated translation sub-rows across cards and within each card.
- **English Input Mode Feature Upgrade (8 Core Specifications)**:
  - **Spec 1 (Single Shift Toggle)**: Seamlessly switch between Chinese and English input modes with a single press of the Shift key.
  - **Spec 2 (Long Text & Pure Space Key)**: In English mode, Space acts purely as a space character (does not auto-commit), allowing fluent typing of sentences and phrases.
  - **Spec 3 (Enter Confirmation)**: Enter confirms and outputs the full typed English text directly.
  - **Spec 4 (Associative Intelligence)**: Full prefix matching and associative word recommendations matching the Chinese mode experience.
  - **Spec 5 (Default Simplified Chinese Follow Translations)**: Candidate cards display Chinese Simplified (ZH) definitions by default.
  - **Spec 6 (Extra Follow Language in Settings)**: Supports selecting an additional follow language (e.g. Japanese, German) alongside Chinese in Settings.
  - **Spec 7 (User Self-Learning & Auto-Promotion)**: Frequently or consecutively typed English words, phrases, and custom sentences are remembered and prioritized to the front on subsequent typing.
  - **Spec 8 (Comprehensive Testing)**: 90 automated tests covering single letters (`a`, `b`, `i`), words (`check`, `moracall`, `went`), collocations, phrases, and long sentences (`what are you doing`).
- **Fixed Consecutive Duplicate Character Dropping (`moracall` -> `moracal`)**:
  - Rewrote the Win32 `SendInput` text injection pipeline in `inject.rs` to emit sequential `[KEYDOWN, KEYUP]` event pairs per character, completely preventing target applications from coalescing consecutive duplicate characters like `ll`, `pp`, or `ee`.
- **Fixed Missing Translations for Chinese Variants (e.g. `妳说什么`)**:
  - Added character variant normalization (`妳` -> `你`, `祂` -> `他`, `牠` -> `它`, `裏` -> `里`, `著` -> `着`), ensuring variant forms inherit translations from standard entries.

## [v1.0.4] - 2026-10-27

### UI Redesign & Modern Beautification
- **Modern Protruding Preedit Pill Tab**:
  - Distinctive protruding preedit pill tab rendered at the top of the candidate window using Win32 GDI region combination (`CombineRgn`), creating a seamless bridge and transparent outer edges.
  - Upgraded main container to a sleek, modern rounded card box.
- **Card-Style Candidates & Accent Blue Capsule**:
  - Each candidate item is housed within an independent rounded card container with fine borders and vertical text hierarchy.
  - The currently highlighted candidate features an accent blue pill capsule (`#0078D7`) with crisp bold white typography.
- **Compact International Language Tags & Settings Pill Badge**:
  - Translation language prefixes streamlined into compact international standard codes (`EN`, `JA`, `DE`, etc.) saving space and improving aesthetics.
  - Sleek dark pill badge in the upper right corner housing the `设置` (Settings) button and `1/3` page indicator.
  - Subtle pagination chevrons (`‹` and `›`).
- **Version Bumped to 1.0.4**:
  - Bumped core, TSF, MSI packaging, and standalone setup installer to version `1.0.4`.

## [v1.0.3] - 2026-10-27

### Added & Improved
- **1-Click MSI Installer**: Added standard Windows `.msi` installer for effortless installation by non-technical users.
- **Project Renamed**: Officially updated project name to **MC-DualType**.
- **Chinese Syllable Capacity & Paging Fixes**:
  - Fixed hard limit of 10 single characters per syllable in `build_dict.py`, increasing capacity to 100 characters per syllable.
  - Resolved missing character `浮` under `fu` (promoted to Candidate #3 on Page 1) and restored full access to `父`, `附`, `覆`, `复`, `扶`, `幅` etc. across multiple pages.
  - Fixed TSF `would_handle` key filter bug to ensure `=` and `-` page-turning keys are properly processed without leaking into host applications.
- **Complete English Alphabet & Irregular Forms**:
  - Restored single-letter completions for all letters `a`..`z` with proper phonetic meanings (e.g. `b` -> `字母B`).
  - Added support for all high-frequency English vocabulary and irregular past tenses / participles (`went`, `took`, `came`, `knew`, `saw`, `got`, `was`, `were`, `been`, `old`, etc.).
  - Deep scanning for short prefixes to prioritize common English words over rare phonetic place names.
- **Bilingual & Multilingual Hierarchical Candidates**: Displays Chinese along with multilingual translations (English, German, Japanese, etc.) hierarchically; supports English-to-Chinese reverse lookup.
- **Single-tap Shift Mode Toggle**: Seamlessly switch between Chinese and ABC English input mode with a single Shift key press, featuring floating status indicator bubbles.
- **Dual Architecture TSF**: Complete Text Services Framework support for both 64-bit and 32-bit Windows host applications.
- **Adaptive Learning**: Context-aware association, dynamic frequency boosting, and automatic user vocabulary learning.

## [v1.0.2]

### Enhancements
- Optimized sentence pinyin segmentation using Viterbi k-best algorithm.
- Enhanced installer scripts and registry cleanups.

## [v1.0.1]

### Improvements
- Integrated CC-CEDICT Chinese-English reverse lookup with offline dictionary.

## [v1.0.0]

### Initial Release
- Initial stable release including standalone hook-based IME and TSF text service.
