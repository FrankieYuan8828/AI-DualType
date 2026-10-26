# 实测 4：conhost 里候选窗出现在哪个位置
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
[DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr h, bool alt);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
"@

$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-CONHOST-TEST4 & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 2000

function Find-ByTitle($needle) {
  $script:found = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    if ([W.T]::IsWindowVisible($h)) {
      $s = New-Object System.Text.StringBuilder 256
      [void][W.T]::GetWindowTextW($h, $s, 256)
      if ($s.ToString() -like "*$needle*") { $script:found = $h; return $false }
    }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:found
}

$hwnd = Find-ByTitle 'MC-IME-CONHOST-TEST4'
if ($hwnd -eq [IntPtr]::Zero) { '未找到窗口'; exit 1 }
$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()

# 先轻敲一下 Alt：前台锁定只对「刚有过用户输入」的进程放开
[System.Windows.Forms.SendKeys]::SendWait('%')
Start-Sleep -Milliseconds 150

[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 600

# 抢焦点可能被系统拦，多试几次直到真的在前台
for ($i = 0; $i -lt 8 -and [W.T]::GetForegroundWindow() -ne $hwnd; $i++) {
  [W.T]::SwitchToThisWindow($hwnd, $true)
  Start-Sleep -Milliseconds 600
}
"前台就是该窗口: $([W.T]::GetForegroundWindow() -eq $hwnd)"
if ([W.T]::GetForegroundWindow() -ne $hwnd) { '抢焦点失败，退出'; exit 1 }

$wr = New-Object W.T+RECT; [void][W.T]::GetWindowRect($hwnd, [ref]$wr)
$cr = New-Object W.T+RECT; [void][W.T]::GetClientRect($hwnd, [ref]$cr)
"cmd 窗口 rect: $($wr.Left),$($wr.Top),$($wr.Right),$($wr.Bottom)"
$cp = New-Object W.T+POINT; [void][W.T]::GetCursorPos([ref]$cp)
"鼠标位置: $($cp.X),$($cp.Y)"

foreach ($ch in 'n','i','h','a','o') {
  [System.Windows.Forms.SendKeys]::SendWait($ch)
  Start-Sleep -Milliseconds 240
}
Start-Sleep -Milliseconds 500

$script:cands = @()
$cb2 = [W.T+EnumProc]{
  param($h, $l)
  $s = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $s, 256)
  if ($s.ToString() -eq 'MCTypeEnglishCandidates') {
    [uint32]$p = 0
    [void][W.T]::GetWindowThreadProcessId($h, [ref]$p)
    $r = New-Object W.T+RECT
    [void][W.T]::GetWindowRect($h, [ref]$r)
    $script:cands += "  候选窗 hwnd=$h pid=$p visible=$([W.T]::IsWindowVisible($h)) rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"
  }
  return $true
}
[void][W.T]::EnumWindows($cb2, [IntPtr]::Zero)
'--- 候选窗 ---'
$script:cands

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save('C:\MC-TypeEnglish\screenshots\conhost4-fullscreen.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()

# 只截控制台窗口本身，方便看清候选窗与提示符的相对位置
$w2 = $wr.Right - $wr.Left; $h2 = $wr.Bottom - $wr.Top
$crop = New-Object System.Drawing.Bitmap($w2, $h2)
$g2 = [System.Drawing.Graphics]::FromImage($crop)
$g2.CopyFromScreen($wr.Left, $wr.Top, 0, 0, (New-Object System.Drawing.Size($w2, $h2)))
$crop.Save('C:\MC-TypeEnglish\screenshots\conhost4-window.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g2.Dispose(); $crop.Dispose()

[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 300
'--- 本次新增日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
