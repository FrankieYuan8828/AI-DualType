# MC-TypeEnglish

![MC-TypeEnglish Banner](image.png)

> 打拼音，候选窗直接给出 **中文 + 多语译文** 的层级候选；打英文，反查中文：
>
> ```text
> nihao
> 1    你好
> 1.1  hallo        （德语）
> 1.2  hello        （英文）
> 1.3  こんにちは    （日语）
> 2    妳好
> 3    你好啊
> ```
>
> 候选是**横向卡片**：上层中文、下层英文，两层共用一个编号。
> 按 `1` 选中「你好」，**再按一次 `1` 切到下面那行英文**，空格上屏当前层；鼠标点哪层就选哪层。
> 每页 7 组，`-` / `+`（或 PageUp / PageDown）翻页。
> **单按 Shift** 快速切换中文与 ABC 英文输入模式（带悬浮状态气泡提示）。
> 越用越懂你：常用词自动提前、上下文联想、生僻词自动学会。
>
> 反过来也行：打 `hello` / `hallo` / `china` 出中文候选（`你好`、`中国`…）。

**长串也能打**：连续拼音会做分词/组句（词频 Viterbi k-best），不是只认整词。

```
bukeyitihuan        → 1 不可以替换   2 不可一替换 …
womenmingtiankaishi → 1 我们明天开始
xiexienile          → 1 谢谢你了
```

**质量是可量化的**（`mcte.exe --eval tools\eval_cases.tsv`，232 条用例）：

| 指标 | 全部用例 | 去掉人工校正表 |
|---|---|---|
| top-1 | 95.7% | 94.6% |
| top-3 | 99.6% | 99.3% |

明细与已知短板见 [docs/EVAL.md](docs/EVAL.md)。

**译文覆盖率**（实测 `--probe` 可查）：

| 数据 | 规模 | 来源 |
|---|---|---|
| 中文 → 英文 | **57,437** 词（几乎任何常用中文词都有英文子项） | CC-CEDICT |
| 英文 → 中文 | **39,850** 个英文词头 | CC-CEDICT 反查 + 手写表 |
| 中文 → 德/日/法/西 | 53 个高频词（`data/glossary.tsv`，可自己加） | 手写 |

Windows 11 输入法，两种形态共用同一套引擎与词库：

| 形态 | 组件 | 出现在 Win+Space | 提权窗口可用 | 需要管理员 |
|---|---|---|---|---|
| **TSF 文本服务**（推荐） | `mcte_tsf.dll` + `mcte_tsf_x86.dll` | ✅ | ✅ | 安装时一次 |
| 钩子版（备用） | `mcte.exe` | ❌ | ❌ | 不需要 |

产品思路来自对 TypeAnything 的逆向（见 [docs/RE-TypeAnything.md](docs/RE-TypeAnything.md)），
TSF 实现细节与踩坑见 [docs/TSF-IMPLEMENTATION.md](docs/TSF-IMPLEMENTATION.md)。

---

## 1. 安装

```text
dist\MC-TypeEnglish\
├─ mcte_tsf.dll          64 位 TSF 文本服务
├─ mcte_tsf_x86.dll      32 位 TSF 文本服务（微信 / 老 Office 等 32 位应用用）
├─ mcte.exe              钩子版（可选，免管理员）
├─ data\dict.txt         离线词库：141,119 条
├─ data\dict_en.tsv      中文→英文：57,437 词（CC-CEDICT）
├─ data\en2zh.tsv        英文→中文反查：39,850 词头
├─ data\glossary.tsv     手写多语词表（德/日/法/西，可自己加）
├─ mcte.ico              输入法图标（语言栏里那个 MC）
├─ tools\mcte-tsf-selftest.exe   COM 自检工具
├─ install.bat           ← 双击安装（会请求管理员权限）
└─ Uninstall.ps1         卸载
```

双击 `install.bat`，安装器会：

1. 把二进制写进 `C:\Program Files\MC-TypeEnglish\bin-<内容哈希>\`（升级不被占用文件卡住）
2. `regsvr32` 注册 64 位文本服务：CLSID + TSF category + 语言配置（Win+Space 里那一项）
3. `SysWOW64\regsvr32` 注册 32 位文本服务（同一 CLSID 的 WOW6432Node 分支）
4. 写默认配置 / 开始菜单快捷方式 / 「应用和功能」卸载项

安装完按 **Win+Space** 选「MC-TypeEnglish」即可。不需要重启，新开的程序立即生效。

只想要免管理员的钩子版：`install.bat -Hook`。
试运行（不改系统）：`Install.ps1 -DryRun`。

卸载：设置 → 应用 → 安装的应用 → MC-TypeEnglish；或运行安装目录的 `Uninstall.ps1`。
配置、用户词库、日志保留在 `%APPDATA%\MC-TypeEnglish`（重装可继续用）。

---

## 2. 用法

| 操作 | 效果 |
|---|---|
| 打 `nihao` | 候选窗：`1 你好`、`1.1 hallo`、`1.2 hello`、`1.3 こんにちは`、`2 妳好`… |
| `1` | 选中第 1 组，落在**中文层** |
| `1` `1` | 同一组再按一次，切到**英文层**；再按切回中文 |
| 空格 / 回车 | 上屏当前高亮那一层（中文或英文） |
| 鼠标点击 | 点上层的字上屏中文，点下层上屏英文 |
| `-` / `+`（或 PageUp/PageDown） | 上下翻页（每页 7 组） |
| 空格 / 回车 | 上屏当前高亮候选 |
| ↑ ↓ ← → | 移动高亮 |
| Esc | 取消这次输入 |
| Backspace | 先退层级标签，再退拼音 |
| `,` `.` `?` `!` 等 | 自动转中文标点并上屏 |
| **Shift**（单按） | **中文 / ABC 英文模式切换**（伴随悬浮状态气泡提示；组合键或按住打大写不触发） |
| **Ctrl+Alt+P** | 中文 / ABC 模式全局热键切换（可在 `config.json` 中自定义） |

**中/英模式切换**：支持**单按 Shift 键**或 **Ctrl+Alt+P** 自由切换中文 / ABC 英文输入模式。切换时会有「中文模式 / ABC 英文模式」浮动状态气泡提示；按住 Shift 输入大写字母或使用系统组合快捷键时经过防误触处理，不会误切换。切英文也可以随时按 Win+Space 切换系统输入法。

---

## 3. 配置说明

本项目秉承轻量无干扰的原则，配置分为 **「候选窗快速设置」** 和 **「高级 JSON 配置文件」**：

### 3.1 候选窗右上角「设置」按钮（界面操作）

候选窗右上角的 **设置** 按钮是一个即点即选的轻量弹窗，**专用于切换第二跟随语言**：
- **英文永远固定跟随**，在此列表中你可以选择额外跟随的一种语言（德语 / 法语 / 日语 / 韩语 / 俄语 / 西班牙语 / 意大利语 / 葡萄牙语），或选择「不额外语言」。
- 选中的语言会作为英文下方的一行出现在候选卡片中：
  ```text
  1 你好
    英文 hello
    德语 hallo        ← 这里就是你选的那一种
  ```
- **点击即生效**，同时自动保存回 `config.json`。
> 💡 *注：为了保证输入法界面的极致纯粹与零干扰，该界面只保留语种单选，不提供多余的输入框或复杂表单。*

### 3.2 高级配置文件（`config.json`）

完整的运行参数保存在 `%APPDATA%\MC-TypeEnglish\config.json` 中（可用记事本等任意文本编辑器直接修改）：
- 钩子版：托盘菜单点击「重新加载配置与词库」即生效；
- TSF 版：下次聚焦窗口激活时生效。

```json
{
  "extra_language": "de",          // 除英文外再跟随一种语言；null = 只跟英文（可由右上角设置自动写入）
  "toggle_hotkey": "Ctrl+Alt+P",   // 备用切换热键
  "max_parents": 9,                // 一级候选数（上限 9，对应数字键）
  "max_children": 4,               // 每个候选最多几行译文
  "web_translate": true,           // 用免费公开翻译端点补全生词译文（免配置、免 API key）
  "prefix_search": true,           // 边打边匹配前缀（nih -> 你好）
  "learn_enabled": true,           // 自学习开关：词频 / 联想 / 生僻词记忆
  "ai_spell_fallback": true,       // 词库查不到时用 AI 兜底并记进用户词库（需开启下方 AI）
  "ai": {
    "enabled": false,              // 【可选高级功能】默认 false 纯离线运行，无需任何 API Key
    "host": "api.deepseek.com",    // 兼容任何 OpenAI 规范的接口端点
    "path": "/v1/chat/completions",
    "model": "deepseek-chat",
    "api_key": ""                  // 需手动在 json 文件中填写（输入法界面不提供也不收集密钥输入框）
  }
}
```

### 3.3 译文与翻译来源

输入法的译文按以下优先级获取：
1. **本地离线词表（首选，秒出）**：`data/glossary.tsv` 手写表 + CC-CEDICT 5.7 万词中英对照；
2. **免费网页公开翻译（兜底）**：本地没有收录的词，后台异步通过公开公共翻译端点补全（结果内存缓存 6 小时，免配置、不需要 API Key）；
3. **可选 AI 模型（进阶）**：若用户自行在 `config.json` 中开启了 `ai.enabled` 并配置了 `api_key`，则优先由大模型提供整句精细翻译与生僻词补全。

网络请求走系统代理（认系统局域网设置及 `HTTPS_PROXY`），4 秒超时静默放弃，**绝对不会阻塞打字与上屏**。

### 装完在 Win+Space 里看不到？

按顺序自查（`verify_winswitch.ps1` 会自动完成前两步的检查）：

```powershell
Get-WinUserLanguageList | ForEach-Object { $_.LanguageTag; $_.InputMethodTips }
```

1. **列表里有没有 MC-TypeEnglish**：只有 `{81D4E9C9-…}`（微软拼音）说明它没进用户列表
   → 以管理员身份重跑 `Install.ps1`（会调 `EnableLanguageProfile` + 补 HKCU 并重启 ctfmon）。
2. **语言选项里确认**：设置 → 时间和语言 → 语言和区域 → 中文(简体) → 语言选项 → 键盘。
3. 仍然没有：注销再登录一次（输入法列表在登录时缓存）。
4. 能看到但不能打字：把 `%APPDATA%\MC-TypeEnglish\mcte.log` 末尾几行发我，
   里面会记 `tsf activate` / `engine unavailable` 这类线索。

### 更新后「没变化」怎么办（重要）

TSF 组件是**加载进每个宿主进程**的 DLL，装完新版本后：

- **新打开的程序**立刻用新版本；
- **已经在运行的程序**（浏览器 / IDE / 微信 / 资源管理器）仍然握着旧 DLL，直到你重启它们；
- 最稳的一次性做法：重启 Windows。

数据文件（`data\*.tsv`）跟着安装目录走，但旧 DLL 可能读不懂新格式，
所以**别在没重启应用的情况下测试新版本**，否则可能看到 `你好:1482` 这种脏候选。

查一下谁还握着旧 DLL：

```powershell
Get-Process | ForEach-Object { $p = $_; try { $p.Modules | Where-Object { $_.ModuleName -like 'mcte_tsf*' } | ForEach-Object { "{0} ({1}) -> {2}" -f $p.ProcessName, $p.Id, $_.FileName } } catch {} }
```

### 终端里（Windows Terminal / VS Code 终端 / MobaXterm）打不出中文？

TSF 是**按窗口**激活的，而且终端类程序用的是 ACP 文本存储：

1. 先在那个终端窗口里按 **Win+Space** 切到 MC-TypeEnglish（任务栏指示器会变）；
2. 切了仍打不出字 → 现在会自动退回 `SendInput` 注入（日志里有一行
   「宿主不支持 TIP 插入，改用 SendInput 注入」）；
3. 如果连切都切不到（Win+Space 里没有它）→ **终端模式**：
   开始菜单 → 「MC-TypeEnglish 终端模式」（等价于 `mcte.exe --terminal-only`）。
   它只在前台是终端类窗口（Windows Terminal / conhost / mintty / VS Code）时接管按键，
   其它窗口一律让给 TSF 版，所以两者可以同时跑、不会抢键。

   实测结论：Windows Terminal 从不会加载我们的 TSF 组件（日志里 0 条 `host=WindowsTerminal.exe`），
   这是终端宿主的限制，不是注册问题 —— 钩子 + `SendInput` 是终端里唯一可靠的路径。

看日志确认是哪个程序在调我们：

```powershell
Select-String -Path "$env:APPDATA\MC-TypeEnglish\mcte.log" -Pattern 'tsf activate' | Select-Object -Last 5
```

### 可执行文件在哪

- `C:\Program Files\MC-TypeEnglish\mcte.exe` —— 命令行工具 + 钩子版（**TSF 版不需要运行它**）
- `C:\Program Files\MC-TypeEnglish\bin-<内容哈希>\mcte_tsf.dll|mcte_tsf_x86.dll` —— TSF 组件
- 开始菜单里有「MC-TypeEnglish 工具（自检·诊断）」直接打开一个能跑这些命令的窗口：

```powershell
mcte.exe --probe nihao                      # 打印候选树
mcte.exe --probe bukeyitihuan                # 看长串组句
mcte.exe --eval tools\eval_cases.tsv         # 跑 232 条评测集
mcte.exe --selftest                          # 自学习 4 条断言
mcte.exe                                     # 启动钩子版（免管理员备用）
```

### 自学习（`%APPDATA%\MC-TypeEnglish\userdb.json`）

三张表，全部来自你自己的输入，不联网：

| 表 | 作用 | 效果 |
|---|---|---|
| `words` | 每个词的选词次数 + 最近使用时间 | 常用词自动提前（对数加权，第 100 次不会把第 1 次压死） |
| `bigrams` | 上一个词 → 这个词的共现次数 | 打完「报告」再打 `bumen`，「部门」优先（上下文联想） |
| `spellings` | 拼音 → 你实际选过的词 | 生僻词/自造词选过一次就进候选，**下次完全离线可用** |

生僻词自动迭代：拼音在词库里查不到时，若开了 AI，后台异步问一次并把结果写进
`spellings`；之后同一个拼音永久离线出词。词库文件本身不需要重建。

验证命令（不发一个键就能看到效果）：

```powershell
mcte.exe --selftest                 # 词频 / 联想 / 生僻词 / 译文不入库，4 条断言
mcte.exe --probe nihao              # 直接看候选树
mcte.exe --learn wanshang 晚上 5     # 教它 5 次，再看候选顺序
```

---

## 4. 架构

```text
core/   mcte-core            纯逻辑 + 候选窗，两个外壳共用
         ├ engine.rs         按键状态机（无 unsafe，41 个单测覆盖）
         ├ dict.rs           拼音 → 汉字（二分前缀查找，141k 条）
         ├ glossary.rs       汉字 → 多语译文（本地，秒出）
         ├ userdb.rs         自学习：词频 / 联想 / 生僻词编码
         ├ ai.rs             可选 AI：译文补全 + 生僻词兜底
         ├ window.rs         候选窗（GDI 自绘，1 / 1.1 / 2 层级编号）
         └ inject.rs         SendInput(KEYEVENTF_UNICODE) 上屏
app/    mcte.exe             WH_KEYBOARD_LL 全局钩子 + 托盘
tsf/    mcte_tsf.dll         TSF 文本服务：ITfKeyEventSink + ITfInsertAtSelection
        mcte-tsf-selftest    不需要管理员就能验证 COM 面（工厂/接口/引用计数）
```

```text
钩子版： 键盘事件 ──▶ hook ──▶ engine ──▶ SendInput(unicode) ──▶ 文档
TSF 版： ITfKeyEventSink::OnKeyDown ──▶ engine
                                  └──▶ ITfEditSession ──▶ ITfInsertAtSelection ──▶ 文档
```

TSF 版是**进程内**组件，所以管理员权限的窗口里照样能用（不受 UIPI 限制），
这是钩子版做不到的；反过来钩子版不需要管理员，装不上时还能兜底。

---

## 5. 已知限制

| 限制 | 说明 |
|---|---|
| 没有 inline preedit | 拼音与候选都画在自绘候选窗里，不写进文档（因此不会污染宿主文本、也不依赖 composition 生命周期） |
| 32 位应用需要 32 位 DLL | 安装器已自动注册；若只手动 `regsvr32` 了 64 位那个，微信等 32 位程序里不会生效 |
| 候选窗跟光标位置靠 `GetGUIThreadInfo` | 少数自绘控件（部分 Electron / 游戏）拿不到光标矩形，会退化为鼠标位置 |
| 词库 14 万条 + 用户自学习 | 极生僻词首次仍需 AI 兜底或手动选一次；没有它，按空格会把拼音原样上屏（不吞字） |
| 英文反查与拼音共用候选列表 | `china` 既是英文也是合法拼音（chi+na），靠词频决定谁在前；纯英文词（`hello`）会直接出中文 |
| 中文→德/日/法/西只有手写词表 | 需要更多语种就往 `data/glossary.tsv` 加行，或打开 AI 让模型补 |
| 升级时旧版本目录可能残留 | 被长驻进程（explorer）占用的旧 `bin-*` 删不掉，重装或重启后自动清 |

---

## 6. 构建与验证

```powershell
# 生成词库（数据源见 tools/build_dict.py 顶部注释）
python tools\build_dict.py

# 编译（x64 + x86）
cargo build --release -p mcte -p mcte-tsf
cargo build --release -p mcte-tsf --target i686-pc-windows-msvc --lib

# 组装发布包（含语法/编码检查与指纹清单）
powershell -ExecutionPolicy Bypass -File tools\stage_dist.ps1

# 测试
cargo test                                 # 41 passed
target\release\mcte.exe --selftest          # 自学习 4 条断言
dist\MC-TypeEnglish\tools\mcte-tsf-selftest.exe dist\MC-TypeEnglish\mcte_tsf.dll   # COM 8 条断言

# 端到端（真按键 + 真上屏 + 截图）
powershell -ExecutionPolicy Bypass -File tools\verify_e2e.ps1   # 钩子版 7 项
powershell -ExecutionPolicy Bypass -File tools\verify_tsf.ps1   # TSF 版 6 项（需先安装）
```

验证留档在 `screenshots/`：`e2e-*`（钩子版）、`tsf-*`（TSF 版真机打字）。

---

## 7. 目录

```text
core/       mcte-core：引擎 / 词库 / 学习 / 候选窗（可单测）
app/        mcte.exe：钩子版外壳
tsf/        mcte_tsf.dll：TSF 文本服务 + COM 自检工具
packaging/  Install.ps1 / Uninstall.ps1 / install.bat
data/       dict.txt（生成物）+ glossary.tsv（可手改）
tools/      build_dict.py / stage_dist.ps1 / deploy_tsf.ps1 / verify_e2e.ps1 / verify_tsf.ps1
docs/       RE-TypeAnything.md（逆向报告）/ TSF-IMPLEMENTATION.md（TSF 实现与踩坑）
dist/       可直接安装的发布包
screenshots/ 验证截图
```

上游致谢：TypeAnything（产品思路与安装链参考）、librime / Weasel（TSF 与拼音引擎范式）、
rime-essay / phrase-pinyin-data / pinyin-data / OpenCC（词库数据）。本项目代码以 MIT 发布。
