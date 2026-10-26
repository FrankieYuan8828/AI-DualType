# 端到端：在真实 conhost 里输入长句，截图候选窗看是否出现整句译文
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace D -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][D.P]::SetProcessDPIAware()
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$app = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --terminal-only --trace'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1500
"mcte pid: $($app.ProcessId)"

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-SENTENCE & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1800

function Find-ByTitle($needle) {
  $script:found = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $s = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetWindowTextW($h, $s, 256)
    if ($s.ToString() -like "*$needle*") { $script:found = $h; return $false }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:found
}

$hwnd = Find-ByTitle 'MC-IME-SENTENCE'
if ($hwnd -eq [IntPtr]::Zero) { '未找到 conhost 窗口'; exit 1 }

# 抢焦点（先轻敲 Alt 解除前台锁定）
[System.Windows.Forms.SendKeys]::SendWait('%')
Start-Sleep -Milliseconds 150
$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 350
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 500
"前台就是 cmd: $([W.T]::GetForegroundWindow() -eq $hwnd)"

# 先把上一轮残留的半截拼音清掉
[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 300

# 输入「长句子并没有正确的翻译」的拼音
foreach ($ch in 'changjuzibingmeiyouzhengquedefanyi'.ToCharArray()) {
  [System.Windows.Forms.SendKeys]::SendWait([string]$ch)
  Start-Sleep -Milliseconds 110
}
"已输入拼音，等翻译返回…"
Start-Sleep -Milliseconds 6000

# 截候选窗
$script:cand = [IntPtr]::Zero
$cb2 = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates' -and [W.T]::IsWindowVisible($h)) { $script:cand = $h; return $false }
  return $true
}
[void][W.T]::EnumWindows($cb2, [IntPtr]::Zero)
$c = $script:cand
if ($c -eq [IntPtr]::Zero) { '候选窗不可见'; }
else {
  $r = New-Object W.T+RECT; [void][W.T]::GetWindowRect($c, [ref]$r)
  $w = $r.Right-$r.Left; $h2 = $r.Bottom-$r.Top
  $bmp = New-Object System.Drawing.Bitmap($w, $h2)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($c, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save('C:\MC-TypeEnglish\screenshots\sentence-candidates.png', [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "候选窗 ${w}x${h2} 已保存 sentence-candidates.png"
}

'--- 本次日志（翻译相关）---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before | Where-Object { $_ -match 'translate:|ai\[|commit:' }
