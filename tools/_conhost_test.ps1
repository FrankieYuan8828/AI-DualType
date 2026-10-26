# 实测：真实 cmd.exe（conhost，ConsoleWindowClass）里能否用输入法
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

function Shot($hwnd, $path) {
  $r = New-Object W.T+RECT
  [void][W.T]::GetWindowRect($hwnd, [ref]$r)
  $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  "saved $path (${w}x${h}) @ $($r.Left),$($r.Top)"
}

$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

# 用 WMI 起独立 cmd 控制台（不受本工具 Job 影响），标题里带标记便于定位窗口
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'cmd.exe /k "title MC-IME-CONHOST-TEST & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
$cmdPid = $res.ProcessId
"cmd pid: $cmdPid"
Start-Sleep -Milliseconds 1500

# conhost 的窗口不属于 cmd.exe，按标题找
$found = [IntPtr]::Zero
$cb = [W.T+EnumProc]{
  param($h, $l)
  if ([W.T]::IsWindowVisible($h)) {
    $sb = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetWindowTextW($h, $sb, 256)
    if ($sb.ToString() -like '*MC-IME-CONHOST-TEST*') { $script:found = $h; return $false }
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
$hwnd = $script:found
if ($hwnd -eq [IntPtr]::Zero) { '未找到 cmd 窗口，退出'; exit 1 }
$sb = New-Object System.Text.StringBuilder 256
[void][W.T]::GetClassNameW($hwnd, $sb, 256)
"窗口类: $($sb.ToString())"

$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 600
"foreground==cmd: $([W.T]::GetForegroundWindow() -eq $hwnd)"

[System.Windows.Forms.SendKeys]::SendWait('nihao')
Start-Sleep -Milliseconds 900
Shot $hwnd 'C:\MC-TypeEnglish\screenshots\conhost-1-composing.png'
[System.Windows.Forms.SendKeys]::SendWait(' ')
Start-Sleep -Milliseconds 1000
Shot $hwnd 'C:\MC-TypeEnglish\screenshots\conhost-2-committed.png'

'--- 本次新增日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
