# 实测 3：强制 conhost（ConsoleWindowClass）窗口，人速输入
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

# 用 conhost.exe 直接托管 cmd，避免被 Windows Terminal 接管
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-CONHOST-TEST3 & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
"launched pid: $($res.ProcessId)"
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

$hwnd = Find-ByTitle 'MC-IME-CONHOST-TEST3'
if ($hwnd -eq [IntPtr]::Zero) { '未找到 conhost 窗口'; exit 1 }
$cls = New-Object System.Text.StringBuilder 256
[void][W.T]::GetClassNameW($hwnd, $cls, 256)
"窗口: $hwnd 类=$($cls.ToString())"

$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 600
"前台是否该窗口: $([W.T]::GetForegroundWindow() -eq $hwnd)"

foreach ($ch in 'n','i','h','a','o') {
  [System.Windows.Forms.SendKeys]::SendWait($ch)
  Start-Sleep -Milliseconds 260
}
Start-Sleep -Milliseconds 400
$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($hwnd, [ref]$r)
$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
$bmp.Save('C:\MC-TypeEnglish\screenshots\conhost3-composing.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"composing 截图: ${w}x${h}"

[System.Windows.Forms.SendKeys]::SendWait(' ')
Start-Sleep -Milliseconds 900
$bmp2 = New-Object System.Drawing.Bitmap($w, $h)
$g2 = [System.Drawing.Graphics]::FromImage($bmp2)
$g2.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
$bmp2.Save('C:\MC-TypeEnglish\screenshots\conhost3-committed.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g2.Dispose(); $bmp2.Dispose()

'--- 本次新增日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
