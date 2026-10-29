# MC-DualType

### 🌏 A Revolutionary Bidirectional Smart Input Method for English Learners & Chinese Beginners

> **A brand-new bidirectional input method designed not only for learning English, but also for foreigners mastering Chinese!**  
> Type Pinyin to receive "Chinese + multilingual translations" in real time. Type English for instant, intelligent Chinese reverse lookups. Seamlessly blends cross-language typing, bilingual translation, and language learning into a single, zero-barrier experience.

![MC-DualType Banner](image.png)

---

### 🚀 Quick Start for Everyone (Recommended Windows MSI Installer)

> **No developer experience, no compiler setup, and no command line needed!**  
> Download the official standard Windows **MSI Installer**. Just double-click to install, and system-level input method registration completes automatically in seconds!

| Package Type | Target Audience | Direct Download Link | Installation Method |
| :--- | :--- | :--- | :--- |
| 📦 **Windows Standard Installer (`.msi`)** | **🌟 Strongly Recommended (First Choice)** | **[👉 Download Latest MSI (v1.3.1)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1.msi)** | **Double-click to run**, fully automated system-level registration |
| ⚡ **Standalone Setup Installer (`Setup.exe`)** | Portable / Single-file Self-elevating | **[👉 Download Latest Setup.exe (v1.3.1)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Setup.exe)** | Self-extracting with auto-elevation |
| 🗜️ **Portable Distribution Archive (`.zip`)** | Portable / Manual Deployment | **[👉 Download Latest Portable.zip (v1.3.1)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Portable.zip)** | Unpack to folder, run `install.bat` |
| 🔗 **All Releases on GitHub** | Release Archives & Notes | **[👉 Browse All Releases on GitHub](https://github.com/FrankieYuan8828/MC-DualType/releases)** | Check past releases and assets |

#### 📖 3 Simple Steps to Start:
1. **Download and double-click** `MC-DualType-1.3.1.msi`.
2. When the Windows User Account Control (UAC) prompt appears, click **"Yes / Allow"**, and wait 2 seconds for installation to complete.
3. Press **`Win + Space`** on your keyboard to switch to **"MC-DualType"**, and type directly in any application (WeChat, QQ, Microsoft Office, Chrome, Edge, IDEs, games, etc.)!
> 💡 **Effortless Uninstallation**: You can cleanly uninstall MC-DualType at any time via Windows **"Settings → Apps → Installed apps"**.

---

## 📦 Version History & Release Packages

Here is the complete version-by-version release breakdown with detailed upgrade/fix descriptions and download packages:

| Version | Release Date | Key Upgrades & Fixes | Download Assets |
| :--- | :--- | :--- | :--- |
| **v1.3.1** | 2026-10-29 | **Critical Engine Freeze Fixes & AI Workbench Overhaul**: Eliminated 27s typing freezes when typing non-exact pinyin combinations like `anxin` (on `x`) or `shili` (on `shil`) by blocking $O(N^3)$ particle decompositions on hybrid English-Chinese entries; continuous typing latency dropped to **< 0.5ms**; documented autonomous learning of missing syllables (`kanma` -> `看吗`); removed AI Workbench dashed frames; enlarged buttons to 42px bold font; mouse hand cursor (`IDC_HAND`); Enter key instant send; live Doubao conversation sync. | [📦 MSI (19.0 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1.msi) <br> [⚡ Setup.exe (72.6 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Setup.exe) <br> [🗜️ Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.1/MC-DualType-1.3.1-Portable.zip) |
| **v1.3.0** | 2026-10-29 | **V1.3 UI Redesign & Doubao-Style Instant AI Workbench**: Streamlined candidate box with left-aligned preedit pill (`BUBBLE_LEFT_X = 34`) and clean symmetric margins (removed redundant sidebar buttons for a distraction-free typing experience); standalone AI Workbench window summoned instantly via global shortcut **`Ctrl + Alt + Space`** with zero reboot required; inner bilingual white card (`[译文] ⇄ [EN] ●`, 1-click copy `📋`, `[原文]`); bottom adaptive dialog input bar + dashed third-party button mapping area (`深度思考`, `联网`, `+上传`, `发送`); robust cursor positioning preventing off-screen drifts in modern apps (Chrome, WeChat, VS Code); CDP bridge for Doubao Desktop (:9444) and DeepSeek browser (:9333); lexicon sharing. | [📦 MSI (19.0 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0.msi) <br> [⚡ Setup.exe (72.6 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Setup.exe) <br> [🗜️ Portable.zip](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.3.0/MC-DualType-1.3.0-Portable.zip) |
| **v1.2.0** | 2026-10-29 | **Lexicon Sharing & Third-Party CDP Integration**: Export/import custom vocabulary packages with high-frequency boosting; settings tab for lexicon sharing; enhanced browser/desktop client CDP bridges. | [📦 MSI (18.8 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.2.0/MC-DualType-1.2.0.msi) <br> [⚡ Setup.exe (72.2 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.2.0/MC-DualType-Setup.exe) |
| **v1.1.2** | 2026-10-28 | **Overlay Fixes, OLE/COM File Upload & Anti-Drift**: Resolved stuck screen overlay with Esc dismissal and 90s self-healing timeout; fixed modal file dialog COM initialization bug (`GetOpenFileNameW`); eliminated hardcoded candidate mock data preventing candidate window drift; TSF build visibility sync. | [📦 MSI (16.9 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.2/MC-DualType-1.1.2.msi) <br> [⚡ Setup.exe](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.2/MC-DualType-Setup.exe) |
| **v1.1.1** | 2026-10-28 | **Settings Crash & Off-screen Layout Fixes**: Resolved Win32 `RefCell` re-entrancy crash during bridge scanning; fixed bottom buttons clipped by window borders under high-DPI scaling (175%); dynamic client-area anchoring; scan button single-flight protection. | [📦 MSI (16.9 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.1/MC-DualType-1.1.1.msi) |
| **v1.1.0** | 2026-10-28 | **AI Access, Multimodal Recognition & Expandable Slider**: AI Access Option 1 (API Key) & Option 2 (Local bridge scanner for Ollama, LM Studio, etc.); multimodal file upload (`+`) & marquee screen snapshot (`📷`); rare character and translation self-upgrading closed loop; expandable slider UI. | [📦 MSI (16.9 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.1.0/MC-DualType-1.1.0.msi) |
| **v1.0.5** | 2026-10-27 | **English Mode Upgrade & Deduplication**: Semantic deduplication of inflected English words; single-tap Shift mode toggle; pure Space typing in English mode; Enter to confirm raw input; fixed consecutive duplicate character dropping (`moracall`); Chinese variant normalization (`妳` -> `你`). | [📦 MSI (5.59 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.0.5/MC-DualType-1.0.5.msi) |
| **v1.0.4** | 2026-10-27 | **UI Modernization**: Protruding preedit pill tab with GDI region combination (`CombineRgn`); card-style candidate containers with accent blue highlighters; international language badges (`EN`, `JA`, `DE`). | [📦 MSI (5.25 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.0.4/MC-DualType-1.0.4.msi) |
| **v1.0.3** | 2026-10-27 | **Official Name Update & 1-Click MSI**: Renamed to MC-DualType; introduced 1-click Windows MSI installer; expanded syllable character capacity to 100 per syllable (`fu` -> `浮`); restored all single-letter English headwords `a`..`z` and irregular verb forms. | [📦 MSI (5.25 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.0.3/MC-DualType-1.0.3.msi) |
| **v1.0.2** | 2026-10-26 | **Sentence Segmentation Enhancement**: Viterbi k-best pinyin sentence segmentation optimization; installer registry cleanup scripts. | [📦 MSI (5.02 MB)](https://github.com/FrankieYuan8828/MC-DualType/releases/download/v1.0.2/MC-TypeEnglish-1.0.2.msi) |
| **v1.0.1** | 2026-10-26 | **Offline Reverse Dictionary**: Integrated CC-CEDICT Chinese-English reverse lookup with offline dictionary engine. | [📦 Release Assets](https://github.com/FrankieYuan8828/MC-DualType/releases) |
| **v1.0.0** | 2026-10-26 | **Initial Public Release**: Dual architecture (TSF text service + Hook fallback); hierarchical bilingual candidate display; local offline learning. | [📦 Release Assets](https://github.com/FrankieYuan8828/MC-DualType/releases/tag/v1.0.0) |

---

## ✨ Core Features & Innovations

### 1. Bidirectional Typing & Learning
- **Type Pinyin, Get Chinese + Multilingual Translations**:
  ```text
  nihao
  1    你好
  1.1  hallo        (German)
  1.2  hello        (English)
  1.3  こんにちは    (Japanese)
  2    妳好
  3    你好啊
  ```
- **Hierarchical Card Interaction**:
  - Press `1` to select candidate #1 (focused on the Chinese line).
  - Press `1` **a second time** to toggle selection to the English translation line.
  - Press `Space` to commit whichever line is active. Mouse clicks directly select your desired tier!
- **Type English, Instant Chinese Reverse Lookup**:
  - Type `hello`, `hallo`, `computer`, or `china` to immediately receive Chinese candidates (`你好`, `计算机`, `中国`…).

### 2. Long Pinyin Sentence Segmentation
Full sentence segmentation using frequency-weighted Viterbi k-best search:
```text
bukeyitihuan        → 1 不可以替换   2 不可一替换 …
womenmingtiankaishi → 1 我们明天开始
xiexienile          → 1 谢谢你了
```

### 3. Quantitative Typing Quality (Evaluated on 232 test cases)
| Metric | Full Benchmark | Without Manual Correction Table |
| :--- | :--- | :--- |
| **top-1 Accuracy** | **95.7%** | **94.6%** |
| **top-3 Accuracy** | **99.6%** | **99.3%** |

*(Detailed breakdown available in [docs/EVAL.md](docs/EVAL.md))*

### 4. Offline Dictionary Scale
| Data Source | Vocabulary Count | Description |
| :--- | :--- | :--- |
| **Chinese → English** | **57,437** entries | CC-CEDICT comprehensive bilingual definitions |
| **English → Chinese** | **39,850** headwords | Instant reverse lookup with inflected forms |
| **Chinese → Multilingual** | German, Japanese, French, Spanish | Expandable via `data/glossary.tsv` or AI |

---

## 🎨 UI Architecture, AI Workbench & Version Screenshots

### 1. Version 1.3.0 (Latest Release): Streamlined Candidate UI & Instant AI Workbench

#### 💡 Streamlined Candidate Box (Pure, Distraction-Free Typing)
![v1.3.0 Candidate Window](screenshots/test-demo-shima.png)
- **Left-aligned Preedit Capsule**: Pinned at `x = 34`, floating cleanly above the candidate container without visual drift.
- **Distraction-Free Architecture**: Removed redundant side buttons (`+` / `📷`) from the primary typing box to eliminate clutter and misclicks. All AI tools are now gracefully integrated into the AI Workbench.
- **Symmetric Pagination**: Balanced margins with refined pagination chevrons `‹` and `›`.

#### 🤖 Doubao-Style Instant AI Workbench (Summon Anytime with `Ctrl + Alt + Space`)
![v1.3.0 Standalone AI Workbench](screenshots/test-workbench-clean.png)
- **Zero Reboot Required**: Works immediately after installation without needing any system reboot.
- **Instant Global Hotkey `Ctrl + Alt + Space`**: Like Doubao's `Alt + Space`, summon the AI Workbench instantly anytime, anywhere without switching input methods!
- **Bilingual Comparison Card**: Inner pure white card with `[译文] ⇄ [EN] ●` status pill, bold headline, auto-wrapped body text, middle dashed divider line, and interactive 1-click clipboard copy button `📋`.
- **Adaptive Dialog Input Bar**: Automatically stretches and resizes with the window width; supports direct keyboard input, Enter to send, Shift+Enter for newline, Esc to close.
- **Dashed Third-Party Button Mapping Area**: Adaptive container wrapped with `#787878` dashed border containing mapped buttons: `深度思考` (Deep Thinking), `联网` (Web Search), `+上传` (Upload), and `发送` (Send).
- **Smart Anti-Drift Positioning**: Automatically anchors the window to the visible area of the active foreground window, avoiding misplaced popups across multi-monitor setups.

---

### 2. Version 1.1.x: Settings & AI Bridge Integration
![v1.1.x Settings & Bridge Scanner](screenshots/settings-v11.1-fixed.png)
- **Dual AI Configuration**: Direct API Key integration (DeepSeek / OpenAI) and local AI service bridge detection (Ollama, LM Studio).
- **High-DPI Adaptation**: Scaled dialog rendering preventing button clipping across 100% to 200% Windows display scaling.

---

### 3. Version 1.0.x: Dual-Architecture Foundation
![v1.0.x Initial Candidate Cards](screenshots/demo-nihao.png)
- **Dual Input Architecture**: Seamless coordination between Windows TSF text service and low-level keyboard hook.
- **Hierarchical Bilingual Cards**: Chinese primary candidate with instant multilingual translations.

---

## ⚙️ Configuration (`config.json`)

Configuration is located at `%APPDATA%\MC-TypeEnglish\config.json`:

```json
{
  "extra_language": "de",          // Second follow language (de/fr/ja/ko/ru/es/it/pt/null)
  "toggle_hotkey": "Ctrl+Alt+P",   // Alternate global toggle hotkey
  "max_parents": 9,                // Maximum primary candidates (1..9)
  "max_children": 4,               // Maximum translation sub-rows per candidate
  "web_translate": true,           // Free public translation fallback for new words
  "prefix_search": true,           // Prefix matching while typing (nih -> 你好)
  "learn_enabled": true,           // Offline self-learning: word frequency, bigram, spelling memory
  "ai_spell_fallback": true,       // AI assistance for rare words
  "ai": {
    "enabled": false,              // Optional AI capability (fully offline by default)
    "base_url": "https://api.deepseek.com/v1",
    "model": "deepseek-chat",
    "api_key": ""
  }
}
```

---

## 🏗️ System Architecture

MC-DualType provides two deployment models sharing the exact same core engine:

| Form Factor | Components | Appears in Win+Space | Elevated Windows Support | Requires Admin |
| :--- | :--- | :--- | :--- | :--- |
| **TSF Text Service (Recommended)** | `mcte_tsf.dll` + `mcte_tsf_x86.dll` | ✅ Yes | ✅ Yes | Once during install |
| **Hook Engine (Fallback)** | `mcte.exe` | ❌ No | ❌ No | ❌ No |

```text
core/   mcte-core            Core pinyin engine, dictionaries, learning & candidate UI
         ├ engine.rs         Keyboard state machine (safe Rust, 41 unit tests)
         ├ dict.rs           Pinyin -> Chinese (binary prefix search, 141k words)
         ├ glossary.rs       Chinese -> Multilingual translations (offline, instant)
         ├ userdb.rs         Self-learning: frequency, bigrams, spelling memory
         ├ ai.rs             AI translation & multimodal assistance
         ├ backend.rs        Unified backend router & third-party button mappings
         ├ workbench.rs      Adaptive independent AI Workbench window
         ├ window.rs         Candidate & slider window (pixel-perfect GDI)
         └ inject.rs         SendInput Unicode text injection
app/    mcte.exe             WH_KEYBOARD_LL global hook + system tray + CLI
tsf/    mcte_tsf.dll         Windows Text Services Framework (x64 & x86)
packaging/                   MSI & Standalone Setup WiX toolchain
```

---

## 🛠️ Build from Source

### Prerequisites
- Windows 10 / 11 (x64)
- Rust stable toolchain with both `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` targets:
  ```powershell
  rustup target add i686-pc-windows-msvc
  ```
- .NET WiX toolset (optional, for MSI packaging):
  ```powershell
  dotnet tool install --global wix
  ```

### Build Commands
```powershell
# 1. Run all workspace unit tests (132 tests)
cargo test --workspace

# 2. Build release binaries (x64 & x86)
cargo build --release -p mcte -p mcte-tsf
cargo build --release -p mcte-tsf --target i686-pc-windows-msvc --lib

# 3. Build MSI & Setup installers into dist/v1.3.0
powershell -ExecutionPolicy Bypass -File tools\build_msi.ps1 -Version 1.3.0
```

---

## 📄 License & Acknowledgments

This project is open-sourced under the **MIT License**.

**Upstream Credits & References**:
- **TypeAnything**: Product concept and packaging inspiration.
- **librime / Weasel**: TSF and Chinese pinyin engine paradigms.
- **CC-CEDICT / rime-essay / phrase-pinyin-data**: Lexical dictionary and frequency data.

---

## 👤 Author

**FrankieYuan8828**
- **Email**: [frankieyuan.yyd@gmail.com](mailto:frankieyuan.yyd@gmail.com)
- **GitHub**: [https://github.com/FrankieYuan8828](https://github.com/FrankieYuan8828)
- **Project Repository**: [https://github.com/FrankieYuan8828/MC-DualType](https://github.com/FrankieYuan8828/MC-DualType)
