# Changelog

> **MC-DualType**: A bidirectional smart input method that makes learning English effortless, and empowers non-native speakers to master and type Chinese with zero learning curve.

All notable changes across all versions are documented in this file, along with direct download links for release packages.

---

## [v1.3.0] - 2026-10-29

### 📦 Downloads for v1.3.0
- **Windows Standard Installer (Recommended)**: [MC-DualType-1.3.0.msi](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0.msi)
- **Standalone Auto-Elevating Installer**: [MC-DualType-1.3.0-Setup.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Setup.exe)
- **Portable Distribution Archive**: [MC-DualType-1.3.0-Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Portable.zip)

### V1.3 UI Redesign & Adaptive AI Workbench
- **Pixel-Perfect Candidate Window Redesign (Matches Design Spec `界面设计/链接AI的输入法界面.png`)**:
  - **Left-Aligned Preedit Pill (`shima`)**: Modern rounded pill tab pinned to `BUBBLE_LEFT_X = 33`, height `40px`, half-floating seamlessly above the candidate container without visual drift.
  - **Vertical Action Toolbar**: Left vertical button column placed inside the main candidate card container (`x = 8..36`, center `x = 22`):
    - `+` Button (`offset_y + 88`): 1-click file/document picker for multimodal AI extraction and bilingual translation.
    - `📷` Button (`offset_y + 137`): 1-click screen crosshair snapshot OCR for instant area capture and translation.
  - **Horizontal Candidate Cards**: `LEFT_PAD = 65` reserved for side buttons, pagination chevrons `‹` and `›`, smooth rounded corners with unnecessary shoulder regions removed.
- **Adaptive Bilingual AI Slider & Independent Workbench (`界面设计/链接AI的滑块输入法界面-2.png`)**:
  - **Inner Bilingual White Card**: Elegant `#FAF9F8` outer container with inner white rounded card (`#FFFFFF`, border `#D9DED2`, radius `18px`).
  - **Status Indicator**: Header featuring `[译文]` pill, bidirectional icon `⇄`, and `[EN] ●` pill with emerald green connection status indicator light.
  - **Bilingual Comparison View**: Clear bold headlines, clean wrapped paragraph typography, middle dashed divider line, and 1-click clipboard copy button `📋` (with temporary "已复制" feedback).
  - **Adaptive Dialog Input Bar**: Bottom-left conversation input bar stretches horizontally to adapt to any window width; supports direct keyboard input, Enter to send, Shift+Enter for newline, and Esc to close.
  - **Dashed Third-Party Button Mapping Area**: Bottom-right container wrapped in `#787878` dashed border with dynamically calculated adaptive width based on active mapped buttons: `深度思考` (Deep Thinking), `联网` (Web Search), `+上传` (Upload), and `发送` (Send).
- **Standalone AI Workbench Window (`--workbench`)**:
  - Dedicated non-blocking Win32 UI window (`McteWorkbench`) accessible from CLI, system tray menu ("打开 AI 工作台"), or by clicking the adaptive input bar / `▢` button.
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
