# 逆向报告：TypeAnything 是怎么做的，我们怎么借的

> 分析对象：`A-cat-with-carrots/TypeAnything`（11 stars）与 `NowakScarlett647/TypeAnything`（同一项目的镜像）。
> 分析方式：clone 源码后通读关键文件（`HEAD` 均为 `13f30a7`），未做二进制反汇编
> —— 该项目 100% 开源，源码级逆向比反汇编信息量更大也更准确。
> 分析时间：本仓库建立当天。

---

## 1. 它是什么

Windows OS 级**风格化输入法**：打拼音 → 空格选出中文 → **按 Enter** → 中文被 AI
改写/翻译成目标语种或风格 → 在任意 Windows 应用里原地替换。

产品定位（`docs/PRODUCT.md`）是一句话："打拼音，落地任何语言、任何圈层、任何风格"。
商业模式为"托管 + 订阅 + BYOK 后门"（`docs/v0.9.0-managed-mode.md`）。

## 2. 技术栈与架构

| 层 | 实现 | 关键文件 |
|---|---|---|
| 输入法框架 | fork [rime/weasel](https://github.com/rime/weasel)（TSF 文本服务，注册为系统输入法） | `third_party/weasel/` |
| 拼音引擎 | [rime/librime](https://github.com/rime/librime) + `luna_pinyin` 词库 | `.../plugins/typeanything/schema/typeanything.schema.yaml` |
| 核心逻辑 | librime 插件（C++，约 800 行） | `.../src/typeanything_processor.cc` |
| 候选窗 UI | weasel 原生 `WeaselPanel`（只有普通候选，无层级） | `WeaselUI/WeaselPanel.cpp` |
| 设置面板 | 独立 `ta-settings.exe` + WebView2 + HTML/CSS | `tools/ta-settings/main.cpp` |
| 安装器 | 单 exe，内嵌全套 weasel 二进制与 schema，UAC 提权 | `tools/ta-installer/main.cpp` |
| 构建 | xmake + MSVC + 预编译 Boost 1.84 | `_build_weasel_xmake.ps1` / `build.bat librime` |

## 3. 关键链路（`typeanything_processor.cc`）

```text
构造时：connect(engine_->context()->commit_notifier, OnCommit)
输入时：OnCommit(ctx) → accumulated_ += ctx->GetCommitText()      // 累积已上屏汉字
按 Enter：ProcessKeyEvent(XK_Return) && !ctx->IsComposing()
        → 读 %APPDATA%\Rime\typeanything_lang.txt 得到目标（"X:name"，X∈A/B/C/D 分类）
        → suppress_capture_ = true
        → std::thread 里 WinHttpPost 打 OpenAI 兼容接口（默认 api.deepseek.com/v1/chat/completions）
返回后：ExtractContent(json) 取译文
        → SetClipboardUtf8(译文)
        → SendBackspaces(原汉字码点数)           // SendInput(VK_BACK) × N
        → SendPaste()                            // SendInput(Ctrl+V)
        → request_id_ 版本校验：期间用户又按了 Enter 就丢弃旧结果
失败分支：401/403 → ShellExecuteW 打开 ta-settings 模型配置页；空响应 → 保留中文并在本地 log
```

值得抄的三点：

1. **异步 + 版本号校验**：HTTP 请求放 worker 线程，用 `request_id_` 丢弃过期结果，打字永不阻塞。
2. **上屏用剪贴板粘贴**而非逐字符 `SendInput`：绕开部分应用的 IME 拦截，中文/emoji 都稳。
3. **本地 log 兜底**：`typeanything_translate.log` 记录 target / status / 解析结果，IME 出问题可自证。

## 4. 它的短板（也就是我们要改的地方）

| 它的做法 | 问题 | 我们的做法 |
|---|---|---|
| Enter 之后才改写，一次只能一个目标 | 想看多语对照得多切一次语言、多按一次 Enter | **候选窗直接分层列出**：`1 你好` → `1.1 hallo` / `1.2 hello` / `1.3 こんにちは` |
| 改写靠"删字 + 粘贴" | 会触发部分应用的只读/富文本副作用，且有可见闪烁 | 候选选中即上屏，`KEYEVENTF_UNICODE` 直接送字符，不删不改 |
| 候选窗是 weasel 原生面板 | 画不了层级编号 | 自绘候选窗（GDI），标签即按键序列 |
| 必须编译 librime（Boost + xmake + MSVC） | 构建重、迭代慢，二次开发门槛高 | 拼音引擎自研（3.3 MB 词库 + 二分前缀查找），纯 Rust，秒级编译 |
| schema YAML 里配 API key | 普通用户会被 YAML 劝退 | `config.json` + 本地词表优先，AI 只是可选加速项 |

## 5. 我们复用了什么、没复用（诚实交代）

**复用（思路层面）**：
- 异步 AI 补全 + 版本校验 + 本地 log 的工程范式；
- 「候选之外的译文由 AI 提供、用户可换任意 OpenAI 兼容 endpoint」的产品设定；
- 安装器要做的事清单（复制 → 注册 → 自启动 → 卸载项 → 启动），改成免管理员的 HKCU 版本。

**没有复用（代码层面）**：
- 没有 fork Weasel / librime，一行 C++ 都没抄。原因见 §4 最后两行：
  本机没有 xmake 与 Boost，编译 librime 一次 30~60 分钟，且 TSF 候选窗无法表达层级编号。
- 因此当前实现不是 TSF 文本服务，而是「托盘常驻 + WH_KEYBOARD_LL 全局钩子」。

**迁移路径**：`engine.rs`（拼音检索 + 候选树 + 按键状态机）是纯逻辑、无 Win32 依赖，
已经有 31 个单测。将来要做真 TSF 输入法时，只需把 `hook.rs` 换成 `ITfKeyEventSink`、
把 `window.rs` 换成 TSF 的 UI 元素实现，state machine 与词库层原样可用。

## 6. 如何复现这次分析

上游代码**没有随本仓库分发**（避免把 31 MB 第三方 clone 混进交付物），复现只需两条命令：

```powershell
git clone --depth 1 https://github.com/A-cat-with-carrots/TypeAnything.git ta1
git clone --depth 1 https://github.com/NowakScarlett647/TypeAnything.git ta2   # 同项目的镜像
```

本文分析的是 `A-cat-with-carrots/TypeAnything` 的 `master`，HEAD 提交 `13f30a7`
（`fix(classify): strip <think>/<thinking>/code-fence blocks before letter scan (#2)`）。
两个 clone 内容一致，`ta1` 多一个 `docs/` 目录。

| 证据 | 文件（clone 后的相对路径） |
|---|---|
| 核心插件（累积 + Enter 触发 + WinHTTP + 删字粘贴） | `third_party/weasel/librime/plugins/typeanything/src/typeanything_processor.cc` |
| schema（词库/模糊音/标点直输/AI provider 配置） | `third_party/weasel/librime/plugins/typeanything/schema/typeanything.schema.yaml` |
| 产品与商业文档 | `docs/PRODUCT.md`、`docs/v0.9.0-managed-mode.md` |
| 安装器主流程（提权/写盘/注册 TSF/部署 schema/自启动） | `tools/ta-installer/main.cpp` |
| 候选窗与托盘菜单改动 | `third_party/weasel/WeaselUI/WeaselPanel.cpp`、`WeaselTSF/WeaselTSF.rc` |

