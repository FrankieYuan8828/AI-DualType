# MC-TypeEnglish TSF 端到端验证（需要先以管理员身份安装）
#
# 链路：轮换输入法（Win+Space）→ 打到靶子文本框 → 断言上屏文本
# 同时检查宿主进程里是否真的加载了 mcte_tsf.dll（TSF 是进程内组件，这一条最硬）。
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\verify_tsf.ps1

param(
    [string]$OutDir = "C:\MC-TypeEnglish\screenshots",
    [int]$SwitchAttempts = 8
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
# 用 TSF 官方接口切换输入法（比反复按 Win+Space 确定得多）
Add-Type -ReferencedAssemblies System.Runtime.InteropServices -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("71C6E74C-0F28-11D8-A82A-00065B84435C"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface ITfInputProcessorProfileMgr {
    void ActivateProfile(uint dwProfileType, ushort langid, ref Guid clsid, ref Guid guidProfile, IntPtr hkl, uint dwFlags);
}

[ComImport, Guid("33c53a50-f456-4884-b049-85fd643ecfed")]
public class TF_InputProcessorProfiles { }

public static class TsfSwitch {
    public static int Activate(Guid clsid, Guid profile, ushort langid) {
        var mgr = (ITfInputProcessorProfileMgr)new TF_InputProcessorProfiles();
        // TF_PROFILETYPE_INPUTPROCESSOR = 1
        mgr.ActivateProfile(1, langid, ref clsid, ref profile, IntPtr.Zero, 0);
        return 0;
    }
}
"@
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win -Name TsfApi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hWnd, System.Text.StringBuilder lpClassName, int nMaxCount);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
public static bool HasVisibleCandidateWindow(uint pid) {
    bool found = false;
    EnumWindows(delegate(IntPtr hWnd, IntPtr l) {
        if (!IsWindowVisible(hWnd)) return true;
        var sb = new System.Text.StringBuilder(256);
        GetClassNameW(hWnd, sb, 256);
        if (sb.ToString() != "MCTypeEnglishCandidates") return true;
        uint winPid;
        GetWindowThreadProcessId(hWnd, out winPid);
        if (winPid == pid) { found = true; return false; }
        return true;
    }, IntPtr.Zero);
    return found;
}
'@

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$results = New-Object System.Collections.ArrayList
function Check([string]$name, [bool]$ok, [string]$detail) {
    if ($ok) { [void]$results.Add("PASS  $name  $detail") } else { [void]$results.Add("FAIL  $name  $detail") }
}
function Pump([int]$ms) {
    $end = (Get-Date).AddMilliseconds($ms)
    while ((Get-Date) -lt $end) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 20 }
}
function Save-Shot([string]$path) {
    $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}
function Switch-InputMethod {
    # Win+Space：轮换已安装的输入法
    [Win.TsfApi]::keybd_event(0x5B, 0, 0, [UIntPtr]::Zero)        # LWIN down
    [Win.TsfApi]::keybd_event(0x20, 0, 0, [UIntPtr]::Zero)        # SPACE down
    [Win.TsfApi]::keybd_event(0x20, 0, 2, [UIntPtr]::Zero)        # SPACE up
    [Win.TsfApi]::keybd_event(0x5B, 0, 2, [UIntPtr]::Zero)        # LWIN up
}
function Module-Loaded([int]$processId) {
    try {
        $proc = Get-Process -Id $processId -ErrorAction Stop
        return [bool]($proc.Modules | Where-Object { $_.ModuleName -like 'mcte_tsf*' })
    } catch { return $false }
}

Write-Host "== 0/5 前置检查 =="
if (Get-Process mcte -ErrorAction SilentlyContinue) {
    Write-Host "  正在停止钩子版（会与 TSF 抢按键）" -ForegroundColor Yellow
    Get-Process mcte | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}
$clsid = '{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}'
$registered = Test-Path "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$clsid\LanguageProfile\0x00000804"
Check "TSF 已注册" $registered "HKLM\SOFTWARE\Microsoft\CTF\TIP\$clsid"
if (-not $registered) {
    Write-Host "  请先以管理员身份运行 dist\MC-TypeEnglish\Install.ps1" -ForegroundColor Yellow
    $results | ForEach-Object { Write-Host $_ }
    exit 1
}

Write-Host "== 1/5 准备靶子文本框 =="
$form = New-Object System.Windows.Forms.Form
$form.Text = "MC-TypeEnglish TSF target"
$form.Width = 900
$form.Height = 240
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$textbox = New-Object System.Windows.Forms.TextBox
$textbox.Multiline = $true
$textbox.Font = New-Object System.Drawing.Font("Microsoft YaHei UI", 16)
$textbox.Dock = 'Fill'
$form.Controls.Add($textbox)
$form.Show()

function Focus-Target {
    for ($i = 0; $i -lt 20; $i++) {
        [void]$form.Activate()
        [void]$textbox.Focus()
        [void][Win.TsfApi]::SetForegroundWindow($form.Handle)
        Pump 80
        if ([Win.TsfApi]::GetForegroundWindow() -eq $form.Handle) { return $true }
    }
    return $false
}
$focused = Focus-Target
Check "靶子窗口拿到焦点" $focused "foreground == form"

Write-Host "== 2/5 用 TSF 接口切到 MC-TypeEnglish =="
$clsidGuid   = [Guid]'{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}'
$profileGuid = [Guid]'{B3D2F0A4-6C81-4E17-9E45-2D8A9C6B4F72}'
$activated = $false
for ($attempt = 1; $attempt -le 3; $attempt++) {
    try { [TsfSwitch]::Activate($clsidGuid, $profileGuid, 0x0804) | Out-Null } catch { Write-Host "  ActivateProfile 异常：$($_.Exception.Message)" }
    Pump 600
    Focus-Target | Out-Null
    $textbox.Clear(); Pump 150
    [System.Windows.Forms.SendKeys]::SendWait("nihao")
    Pump 900
    $ours = [Win.TsfApi]::HasVisibleCandidateWindow([uint32]$PID)
    Write-Host ("  第 {0} 次：textbox='{1}' 候选窗={2}" -f $attempt, $textbox.Text, $ours)
    if ($ours) { $activated = $true; break }
    [System.Windows.Forms.SendKeys]::SendWait("{ESC}"); Pump 150
}
Check "TSF 服务被加载进宿主进程" $activated "宿主进程模块含 mcte_tsf.dll（候选窗已出现）"

if (-not $activated) {
    Save-Shot (Join-Path $OutDir "tsf-failed.png")
    $form.Close()
    Write-Host ""
    $results | ForEach-Object { Write-Host $_ }
    Write-Host "未能切换到 MC-TypeEnglish：请手动按 Win+Space 选中它，然后重跑本脚本。" -ForegroundColor Yellow
    exit 1
}

Write-Host "== 3/5 拼音 → 候选 → 上屏 你好 =="
[System.Windows.Forms.SendKeys]::SendWait("{ESC}"); Pump 200
$textbox.Clear(); Pump 200
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Pump 900
Save-Shot (Join-Path $OutDir "tsf-1-candidates.png")
[System.Windows.Forms.SendKeys]::SendWait(" ")
Pump 900
Check "空格上屏 你好" ($textbox.Text -ceq '你好') "(textbox='$($textbox.Text)')"

Write-Host "== 4/5 连按 1 切到英文层 → hello =="
[System.Windows.Forms.SendKeys]::SendWait("{ESC}"); Pump 200
$textbox.Clear(); Pump 200
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Pump 800
[System.Windows.Forms.SendKeys]::SendWait("1")   # 中文层
Pump 250
[System.Windows.Forms.SendKeys]::SendWait("1")   # 切到英文层
Pump 250
[System.Windows.Forms.SendKeys]::SendWait(" ")   # 上屏当前层
Pump 900
Check "1 1 空格上屏 hello" ($textbox.Text -ceq 'hello') "(textbox='$($textbox.Text)')"
Save-Shot (Join-Path $OutDir "tsf-2-committed.png")

Write-Host "== 5/5 Ctrl+Alt+P 切到英文，字母直通 =="
[System.Windows.Forms.SendKeys]::SendWait("{ESC}")
Pump 200
function Send-ToggleHotkey {
    [Win.TsfApi]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)   # Ctrl down
    [Win.TsfApi]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)   # Alt down
    [Win.TsfApi]::keybd_event(0x50, 0, 0, [UIntPtr]::Zero)   # P down
    [Win.TsfApi]::keybd_event(0x50, 0, 2, [UIntPtr]::Zero)   # P up
    [Win.TsfApi]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)   # Alt up
    [Win.TsfApi]::keybd_event(0x11, 0, 2, [UIntPtr]::Zero)   # Ctrl up
}
Send-ToggleHotkey
Pump 600
$textbox.Clear(); Pump 200
[System.Windows.Forms.SendKeys]::SendWait("hello")
Pump 700
Check "Ctrl+Alt+P 后英文直通" ($textbox.Text -ceq 'hello') "(textbox='$($textbox.Text)')"
Send-ToggleHotkey
Pump 400

$form.Close()

Write-Host ""
Write-Host "==== 验证结果 ===="
$results | ForEach-Object { Write-Host $_ }
$failed = @($results | Where-Object { $_ -like 'FAIL*' }).Count
if ($failed -gt 0) { Write-Host "FAILED ($failed of $($results.Count))"; exit 1 }
Write-Host "ALL PASS ($($results.Count) checks)"
exit 0
