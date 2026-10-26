# 验证「Win+Space 真的能切到 MC-TypeEnglish 并正常打字」
#
# 与 verify_tsf.ps1 的区别：那个用 TSF 接口确定性地激活（验证功能），
# 这个用**真实 Win+Space 按键**（验证用户手上那条路）。
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\verify_winswitch.ps1

param([string]$OutDir = "C:\MC-TypeEnglish\screenshots")

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win -Name SwitchApi -MemberDefinition @'
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
function Send-WinSpace {
    [Win.SwitchApi]::keybd_event(0x5B, 0, 0, [UIntPtr]::Zero)   # LWIN down
    [Win.SwitchApi]::keybd_event(0x20, 0, 0, [UIntPtr]::Zero)   # SPACE down
    [Win.SwitchApi]::keybd_event(0x20, 0, 2, [UIntPtr]::Zero)   # SPACE up
    [Win.SwitchApi]::keybd_event(0x5B, 0, 2, [UIntPtr]::Zero)   # LWIN up
}

Write-Host "== 0/4 前置：确认用户输入法列表里有 MC-TypeEnglish =="
$list = Get-WinUserLanguageList
$ours = $list.InputMethodTips | Where-Object { $_ -like '*7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31*' }
Check "Win+Space 列表包含 MC-TypeEnglish" ($null -ne $ours) "tips=$ours"
if ($null -eq $ours) {
    Write-Host "  请先以管理员身份运行 dist\MC-TypeEnglish\Install.ps1" -ForegroundColor Yellow
    $results | ForEach-Object { Write-Host $_ }
    exit 1
}

Write-Host "== 1/4 打开靶子文本框 =="
$form = New-Object System.Windows.Forms.Form
$form.Text = "MC-TypeEnglish Win+Space test"
$form.Width = 880; $form.Height = 220
$form.StartPosition = 'CenterScreen'; $form.TopMost = $true
$textbox = New-Object System.Windows.Forms.TextBox
$textbox.Multiline = $true
$textbox.Font = New-Object System.Drawing.Font("Microsoft YaHei UI", 16)
$textbox.Dock = 'Fill'
$form.Controls.Add($textbox)
$form.Show()
[void]$form.Activate(); [void]$textbox.Focus()
[void][Win.SwitchApi]::SetForegroundWindow($form.Handle)
Pump 500

Write-Host "== 2/4 按 Win+Space 轮换，直到打拼音出现我们的候选窗 =="
$switched = $false
for ($i = 1; $i -le 4; $i++) {
    Send-WinSpace
    Pump 900
    [void]$form.Activate(); [void]$textbox.Focus()
    [void][Win.SwitchApi]::SetForegroundWindow($form.Handle)
    Pump 200
    $textbox.Clear()
    [System.Windows.Forms.SendKeys]::SendWait("nihao")
    Pump 800
    $shown = [Win.SwitchApi]::HasVisibleCandidateWindow([uint32]$PID)
    Write-Host ("  第 {0} 次 Win+Space：候选窗={1} textbox='{2}'" -f $i, $shown, $textbox.Text)
    if ($shown) { $switched = $true; break }
    [System.Windows.Forms.SendKeys]::SendWait("{ESC}"); Pump 200
}
Check "Win+Space 能切到 MC-TypeEnglish" $switched "候选窗出现即判定切换成功"
Save-Shot (Join-Path $OutDir "winswitch-1-candidates.png")

if ($switched) {
    Write-Host "== 3/4 空格上屏 =="
    [System.Windows.Forms.SendKeys]::SendWait(" ")
    Pump 800
    Check "空格上屏 你好" ($textbox.Text -ceq '你好') "(textbox='$($textbox.Text)')"
    Save-Shot (Join-Path $OutDir "winswitch-2-committed.png")

    Write-Host "== 4/4 层级候选 1.1 =="
    [System.Windows.Forms.SendKeys]::SendWait("{ESC}"); Pump 150
    $textbox.Clear(); Pump 150
    [System.Windows.Forms.SendKeys]::SendWait("nihao"); Pump 700
    [System.Windows.Forms.SendKeys]::SendWait("1.1"); Pump 800
    Check "1.1 上屏 hallo" ($textbox.Text -ceq 'hallo') "(textbox='$($textbox.Text)')"
}

$form.Close()
Write-Host ""
Write-Host "==== 验证结果 ===="
$results | ForEach-Object { Write-Host $_ }
$failed = @($results | Where-Object { $_ -like 'FAIL*' }).Count
if ($failed -gt 0) { Write-Host "FAILED ($failed of $($results.Count))"; exit 1 }
Write-Host "ALL PASS ($($results.Count) checks)"
exit 0
