# TSF 文本服务实现说明

> 目标：让 MC-TypeEnglish 成为**真正的 Windows 输入法**——出现在 Win+Space 的语言列表里、
> 能在管理员权限的窗口里输入。本文记录实现结构、与 Weasel（TypeAnything 的上游）的对照，
> 以及实际踩到的坑。

---

## 1. 组成

```text
tsf/src/lib.rs       4 个 DLL 导出 + IClassFactory + 引用计数
tsf/src/service.rs   TextService(ITfTextInputProcessorEx) / KeySink(ITfKeyEventSink)
                     / InsertSession(ITfEditSession)
tsf/src/registry.rs  DllRegisterServer 的三件事：CLSID + category + language profile
tsf/src/bin/selftest.rs  不需要管理员就能验证 COM 面
```

**导出函数**（Rust 不需要 .def，`#[no_mangle] extern "system"` 即可）：

```text
DllGetClassObject   DllCanUnloadNow   DllRegisterServer   DllUnregisterServer
```

引用计数语义与 ATL 一致：`g_cRefDll` 从 -1 起算，`DllCanUnloadNow` 在 `>= 0` 时返回 `S_FALSE`。
`TextService` / `KeySink` 构造时 +1、`Drop` 时 -1，两端配对。

**为什么拆成两个对象**：`ITfKeystrokeMgr::AdviseKeyEventSink(tid, sink, TRUE)` 需要传"自己"这个
`ITfKeyEventSink` 指针，而在 windows-rs 的 `#[implement]` 里拿不到外层 COM 指针。
拆开后 `KeySink` 是独立对象，通过 `Arc<Mutex<State>>` 与 `TextService` 共享引擎状态。

**关键版本细节**：windows-rs 0.62 里接口 trait 必须实现在宏生成的 `<类型>_Impl` 上
（`impl ITfKeyEventSink_Impl for KeySink_Impl`），内部状态用 `IUnknownImpl::get_impl()` 取。

---

## 2. 生命周期

```text
Activate/ActivateEx(ptim, tid)
  ├─ 解析数据目录（相对 DLL：同级 data\ 或上一级 data\，见 §5 坑 3）
  ├─ 懒加载引擎：首次激活才读 3.4 MB 词库（141,119 条）
  ├─ QI ITfKeystrokeMgr → AdviseKeyEventSink(tid, sink, fForeground=TRUE)
  └─ PreserveKey(Ctrl+Alt+P)                     ← 中/英切换
Deactivate()
  ├─ UnpreserveKey / UnadviseKeyEventSink
  ├─ flush 用户词库
  └─ 隐藏候选窗
```

按键路径：

```text
OnTestKeyDown  → engine.would_handle(key)   （纯只读、便宜，可被重复调用）
OnKeyDown      → engine.handle(key) → Commit(text) 时：
                   ITfContext::RequestEditSession(TF_ES_SYNC|TF_ES_READWRITE)
                     └─ InsertSession::DoEditSession(ec)
                          └─ QI ITfInsertAtSelection → InsertTextAtSelection(ec, 0, units)
OnPreservedKey → GUID == 我们的 → toggle 中/英
```

**刻意不用 composition**：拼音与候选都画在自绘候选窗里，文档里只出现最终结果。
好处是不用管 `ITfComposition` / `ITfCompositionSink` 的生命周期（Weasel 在这方面花了
大量代码，见 §4 坑 5），也不会因为外部编辑把 composition 打断而崩。

---

## 3. 注册

`DllRegisterServer`（需要管理员，写 HKLM）：

1. `HKCR\CLSID\{7C3E0C11-…}\InprocServer32` → DLL 路径，`ThreadingModel = Apartment`
2. `ITfCategoryMgr::RegisterCategory` × 5：`GUID_TFCAT_CATEGORY_OF_TIP`、`GUID_TFCAT_TIP_KEYBOARD`
   以及 `TIPCAP_SECUREMODE / INPUTMODECOMPARTMENT / UIELEMENTENABLED`
3. `ITfInputProcessorProfileMgr::RegisterProfile(CLSID, 0x0804, GUID_PROFILE, 描述, 图标路径, …)`
   —— 这一步决定 Win+Space 里是否出现、显示什么名字

32 位宿主：同一个 CLSID 在 `HKLM\SOFTWARE\WOW6432Node\Classes\CLSID\…\InprocServer32`
指向 32 位 DLL。安装器用 `SysWOW64\regsvr32` 注册一次即可（32 位 regsvr32 自动写 WOW6432Node）。
profile / category 是机器级、与位数无关，只在 64 位侧注册一次（代码里用
`#[cfg(target_pointer_width = "64")]` 隔开）。

---

## 4. 从 Weasel 学到 / 没照抄的东西

参考实现：`third_party/weasel/WeaselTSF/`（分析记录见 [RE-TypeAnything.md](RE-TypeAnything.md)）。

照搬的：
- 导出函数集合与引用计数语义；
- `InprocServer32` + `ThreadingModel=Apartment` 的注册写法；
- `AdviseKeyEventSink(tid, sink, TRUE)`（没有 cookie，解绑按 tid `UnadviseKeyEventSink`）；
- 文本插入必须走编辑会话的 `ITfInsertAtSelection`。

**没照抄的坑**（Weasel 里真实存在）：

| # | 上游问题 | 我们的做法 |
|---|---|---|
| 1 | `UnregisterServer` 里 TIP 键前缀拼成 `Microsft`，卸载删不掉 | 写对 `Software\Microsoft\CTF\TIP` |
| 2 | `RegisterProfile` 的 `cchDesc/cchFile` 传字节数，中文描述被截断 | 传字符数（windows-rs 直接用 `&[u16]`） |
| 3 | `Deactivate` 漏了 `UninitThreadFocusSink`，且 `UninitThreadMgrEventSink` 调了两次 | 收支成对，且我们根本不 advise 那些 sink |
| 4 | 把 TSF 对象自己当 `ITfEditSession`，异步会话隐式持有对象 | 编辑会话是独立对象，只持有 context + 文本 |
| 5 | composition 的 `OnCompositionTerminated` 在正常结束时也会回调 | 不用 composition，问题不存在 |

---

## 5. 实际踩到的坑（都是本次开发的真实记录）

1. **`ITfSource::AdviseSink` 在 windows-rs 0.62 里签名是错的**
   （第三个参数该是 `DWORD* pdwCookie`，crate 里写成 `dwmask: u32` 按值传）。
   规避：我们不 advise thread-manager / text-edit sink——`ITfKeyEventSink` 从
   `OnKeyDown` 的 `pic` 参数直接拿 context，用不上 cookie。

2. **TSF 不会把修饰键送进 key sink**。单按 Shift 想切换中/英，`OnKeyDown(VK_SHIFT)` 根本
   不会被调用（日志里没有任何记录）。改用 `ITfKeystrokeMgr::PreserveKey` 注册
   Ctrl+Alt+P + `OnPreservedKey` 回调，实测触发正常。

3. **数据目录不能按 `current_exe()` 解析**。TSF 是进程内组件，`current_exe()` 是宿主
   （notepad.exe / powershell.exe）。而安装布局是 `<InstallDir>\bin-<hash>\mcte_tsf.dll`
   + `<InstallDir>\data\`，所以要先找同级 `data\`，找不到再找上一级——早期版本只找同级，
   结果「输入法能激活但一个候选都没有」，日志里是 `engine unavailable: 无法加载词库 …`。

4. **同一线程可能有多个 TIP 实例**（TSF 会重复 Activate）。每个实例各建一个引擎会导致
   「候选窗显示 A 引擎的结果、按键喂给 B 引擎」。改为 `thread_local` 共享一份引擎状态，
   并把 `Activate` 做成幂等（`client_id == tid && engine.is_some()` 直接返回）。

5. **DLL 被长驻进程占用导致升级失败**。`explorer.exe` 会一直映射着已注册的 TSF DLL，
   直接覆盖 `mcte_tsf.dll` 会报「正由另一进程使用」。改为**按内容哈希的版本目录**
   （`bin-<sha8>`）+ 每次注册指向新文件，旧目录在重装/重启后清理。

6. **进程退出时 TLS 已析构**。候选窗状态存在 `thread_local` 里，宿主退出阶段仍可能有回调，
   `LocalKey::with` 会 panic（`cannot access a Thread Local Storage value during or after
   destruction`）。所有 TLS 访问改成 `try_with` + `try_borrow`，析构期静默放弃。

7. **候选窗查重不能只用 `FindWindow`**。同一个类名在多个宿主进程里都有（explorer 也有一份），
   `FindWindow` 可能返回别人那个隐藏窗口。自动化验证改用 `EnumWindows` + 类名 + 进程 id 匹配。

---

8. **`RegisterProfile` 只写机器级，Win+Space 轮的是用户级列表**（这条最坑，表现为「装好了一切正常，
   但 Win+Space 里根本没有它」）：

   | 位置 | 谁写 | 作用 |
   |---|---|---|
   | `HKLM\SOFTWARE\Microsoft\CTF\TIP\{clsid}\LanguageProfile\0x00000804\{profile}` | `RegisterProfile` | 让系统知道这个 IME 存在（Enable=1） |
   | `HKCU\Control Panel\International\User Profile\zh-Hans-CN` 里「**值名 = tips 字符串**、值 = 1」 | `EnableLanguageProfile` / 设置界面 | **Win+Space 实际轮换的列表** |

   只做第一步时，注册表看起来一切正常，但语言栏里没有它。修复方式（本项目已内置）：
   `DllRegisterServer` 里补调 `ITfInputProcessorProfiles::EnableLanguageProfile(..., TRUE)`
   与 `EnableLanguageProfileByDefault`，安装器再兜底写一次 HKCU 的 tips 值并重启 `ctfmon`。

   顺带一个易错点：HKCU 那条数据的**值名是 tips 字符串、值是 DWORD 1**，写成 `"1" = "0804:{...}"`
   是无效的（我第一版就写反了，`Get-WinUserLanguageList` 仍然只有一项）。

9. **图标要单独给 .ico 文件**。`RegisterProfile` 的 `pchIconFile` 指向 DLL 时，Windows 会去
   DLL 资源里找图标；纯 Rust cdylib 没有 icon 资源，语言栏里就显示系统默认的「简」字徽章，
   和微软拼音长得一样、分不出来。修法是把 `IconFile` 指向一个真实的 `.ico`
   （本项目 `mcte.ico`，9 个尺寸，用 `tools/make_icon.ps1` 生成），重新注册即可生效。

10. **终端类宿主（Windows Terminal）两条坑**：
   1. TSF 是**按窗口激活**的，切输入法必须在那个窗口里按 Win+Space（在本终端里没切过，
      日志里就一条 `pid=<终端>` 的激活都没有 —— 这是最常见的原因）；
   2. 即便切过去了，终端用的是 ACP 文本存储，对第三方 TIP 的 `ITfInsertAtSelection`
      支持不全：能吃掉按键、却插不进字。现在 `commit_text()` 会在插入失败时
      **自动退回 `SendInput(KEYEVENTF_UNICODE)`**，并在日志里写一条
      「宿主不支持 TIP 插入，改用 SendInput 注入」。
   诊断时看激活日志里的 `host=` 字段即可知道是哪个程序在调我们，例如
   `tsf activate: host=WindowsTerminal.exe pid=1780 ...`。

## 6. 自检与验证

`tools\mcte-tsf-selftest.exe <dll>`（**不需要管理员**）逐条断言：

```text
PASS  导出函数齐全            DllGetClassObject / DllCanUnloadNow / DllRegisterServer / DllUnregisterServer
PASS  无对象时可以卸载        DllCanUnloadNow = S_OK
PASS  DllGetClassObject 返回类工厂
PASS  CreateInstance 创建 ITfTextInputProcessorEx
PASS  有活对象时拒绝卸载      DllCanUnloadNow = S_FALSE
PASS  ITfTextInputProcessorEx 可 QI 到 ITfTextInputProcessor
PASS  未实现的接口被正确拒绝
PASS  释放后回到可卸载状态    DllCanUnloadNow = S_OK
```

`tools\verify_tsf.ps1`（需先安装）用 `ITfInputProcessorProfileMgr::ActivateProfile`
把当前线程切到 MC-TypeEnglish，然后真打键、真断言：

```text
PASS  TSF 已注册（HKLM\SOFTWARE\Microsoft\CTF\TIP\{…}\LanguageProfile\0x00000804）
PASS  靶子窗口拿到焦点
PASS  TSF 服务被加载进宿主进程（候选窗已出现）
PASS  空格上屏 你好
PASS  1.1 上屏 hallo
PASS  Ctrl+Alt+P 后英文直通
```

`tools\verify_winswitch.ps1` 走的是**用户手上那条路**——真按 Win+Space：

```text
PASS  Win+Space 列表包含 MC-TypeEnglish   tips=0804:{7C3E0C11-…}{B3D2F0A4-…}
PASS  Win+Space 能切到 MC-TypeEnglish     (第 2 次 Win+Space 后候选窗出现)
PASS  空格上屏 你好
PASS  1.1 上屏 hallo
```

---

## 7. 提权窗口为什么可用

TSF 文本服务是**加载进宿主进程**的 in-proc COM 组件：管理员权限的程序（例如以管理员运行的
记事本）同样会把 `mcte_tsf.dll` 加载到自己进程里，按键与插入文本都发生在该进程内部，
不经过跨完整性级别的 `SendInput`，所以不受 UIPI 限制。

（钩子版做不到这一点：低级键盘钩子投递不到更高完整性级别的窗口，注入了也会被拦。）

**验证状态**：自动化按键验证是在普通权限的宿主里完成的（`verify_tsf.ps1` 6/6 通过）。
提权宿主的验证受限于 UIPI——非提权脚本无法向提权窗口注入按键，因此这一项
只做了架构层面的说明，没有留自动化证据。
