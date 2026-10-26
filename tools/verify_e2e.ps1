# MC-TypeEnglish 端到端验证（自动化断言 + 截图）
#
# 链路：脚本开一个 WinForms 文本框当靶子 -> SendKeys 真打拼音 -> 全局钩子 ->
# 候选树上屏。两条独立证据：
#   1) 文本框内容（证明文本真的落进了普通应用）
#   2) %APPDATA%\MC-TypeEnglish\mcte.log 里的 commit 记录（证明是 IME 干的活）
#
# 用法:
#   powershell -ExecutionPolicy Bypass -File tools\verify_e2e.ps1
#   powershell -ExecutionPolicy Bypass -File tools\verify_e2e.ps1 -Exe target\release\mcte.exe

param(
    [string]$Exe = "C:\MC-TypeEnglish\target\debug\mcte.exe",
    [string]$OutDir = "C:\MC-TypeEnglish\screenshots"
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win -Name Api -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
'@

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$logPath = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'

function Save-Shot([string]$path) {
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

function Pump([int]$ms) {
    $end = (Get-Date).AddMilliseconds($ms)
    while ((Get-Date) -lt $end) {
        [System.Windows.Forms.Application]::DoEvents()
        Start-Sleep -Milliseconds 20
    }
}

function Read-NewLog([int64]$offset) {
    if (-not (Test-Path $logPath)) { return '' }
    $stream = [IO.File]::Open($logPath, 'Open', 'Read', 'ReadWrite')
    try {
        $length = $stream.Length
        if ($length -le $offset) { return '' }
        $stream.Seek($offset, 'Begin') | Out-Null
        $buffer = New-Object byte[] ($length - $offset)
        $read = $stream.Read($buffer, 0, $buffer.Length)
        return [Text.Encoding]::UTF8.GetString($buffer, 0, $read)
    } finally { $stream.Dispose() }
}

$results = New-Object System.Collections.ArrayList
function Check([string]$name, [bool]$ok, [string]$detail) {
    if ($ok) { [void]$results.Add("PASS  $name  $detail") }
    else { [void]$results.Add("FAIL  $name  $detail") }
}

Write-Host "== 1/7 启动输入法常驻进程 =="
$before = 0
if (Test-Path $logPath) { $before = (Get-Item $logPath).Length }
$ime = Start-Process -FilePath $Exe -PassThru -WindowStyle Hidden
Pump 2000
if ($ime.HasExited) { throw "输入法进程提前退出，请查看 $logPath" }

Write-Host "== 2/7 准备靶子文本框并抢焦点 =="
$form = New-Object System.Windows.Forms.Form
$form.Text = "MC-TypeEnglish e2e target"
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
for ($i = 0; $i -lt 20; $i++) {
    $form.Activate()
    $textbox.Focus()
    [Win.Api]::SetForegroundWindow($form.Handle) | Out-Null
    Pump 100
    if ([Win.Api]::GetForegroundWindow() -eq $form.Handle) { break }
}
$focused = ([Win.Api]::GetForegroundWindow() -eq $form.Handle)
Check "靶子窗口拿到焦点" $focused "(foreground == form: $focused)"
Pump 300

Write-Host "== 3/7 真打拼音 nihao =="
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Pump 1000
Check "拼音没落进文档" ($textbox.Text -eq '') "(textbox='$($textbox.Text)')"
Save-Shot (Join-Path $OutDir "e2e-1-candidates.png")

Write-Host "== 4/7 真打层级标签 1.1（德语 hallo）=="
$offset = (Get-Item $logPath).Length
[System.Windows.Forms.SendKeys]::SendWait("1.1")
Pump 1000
$log4 = Read-NewLog $offset
Check "1.1 上屏 hallo" ($textbox.Text -eq 'hallo') "(textbox='$($textbox.Text)')"
Check "IME 记录了 commit hallo" ($log4 -match 'commit: "hallo"') "(log: $($log4.Trim()))"
Save-Shot (Join-Path $OutDir "e2e-2-committed.png")

Write-Host "== 5/7 再打 nihao + 空格（中文 你好）=="
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Pump 900
[System.Windows.Forms.SendKeys]::SendWait(" ")
Pump 900
Check "空格上屏 你好" ($textbox.Text -ceq 'hello你好') "(textbox='$($textbox.Text)')"
Save-Shot (Join-Path $OutDir "e2e-3-space.png")

Write-Host "== 6/8 再打 zhongguo + 空格（多字词走空格上屏）=="
[System.Windows.Forms.SendKeys]::SendWait("zhongguo")
Pump 900
[System.Windows.Forms.SendKeys]::SendWait(" ")
Pump 900
Check "空格上屏 中国" ($textbox.Text -ceq 'hello你好中国') "(textbox='$($textbox.Text)')"
Save-Shot (Join-Path $OutDir "e2e-4-space-word.png")

Write-Host "== 7/8 再打 wanshang + 数字 1（没有译文的候选直接上屏）=="
[System.Windows.Forms.SendKeys]::SendWait("wanshang")
Pump 900
[System.Windows.Forms.SendKeys]::SendWait("1")   # 无译文：停在中文层
Pump 250
[System.Windows.Forms.SendKeys]::SendWait(" ")
Pump 900
Check "数字键+空格上屏 晚上" ($textbox.Text -ceq 'hello你好中国晚上') "(textbox='$($textbox.Text)')"
Save-Shot (Join-Path $OutDir "e2e-5-digit.png")

Write-Host "== 8/8 清理 =="
$form.Close()
Stop-Process -Id $ime.Id -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "==== 验证结果 ===="
$results | ForEach-Object { Write-Host $_ }
$failed = @($results | Where-Object { $_ -like 'FAIL*' }).Count
if ($failed -gt 0) { Write-Host "FAILED ($failed of $($results.Count))"; exit 1 }
Write-Host "ALL PASS ($($results.Count) checks)"
exit 0
