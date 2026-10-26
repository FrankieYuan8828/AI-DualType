# 实测 2：逐键输入，实时查询候选窗口是否可见及位置
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

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'cmd.exe /k "title MC-IME-CONHOST-TEST2 & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
"cmd pid: $($res.ProcessId)"
Start-Sleep -Milliseconds 1500

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

$hwnd = Find-ByTitle 'MC-IME-CONHOST-TEST2'
$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 500

function Probe($tag) {
  $c = Find-ByTitle 'MC-TypeEnglish'   # 候选窗标题
  $r = New-Object W.T+RECT
  if ($c -ne [IntPtr]::Zero) {
    [void][W.T]::GetWindowRect($c, [ref]$r)
    $sb = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($c, $sb, 256)
    "  [$tag] cand hwnd=$c class=$($sb.ToString()) visible=$([W.T]::IsWindowVisible($c)) rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"
  } else {
    "  [$tag] cand window NOT FOUND"
  }
}

Probe 'before'
foreach ($ch in 'n','i','h','a','o') {
  [System.Windows.Forms.SendKeys]::SendWait($ch)
  Start-Sleep -Milliseconds 220
  Probe "after '$ch'"
}

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save('C:\MC-TypeEnglish\screenshots\conhost-3-fullscreen.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
'  fullscreen saved'

[System.Windows.Forms.SendKeys]::SendWait(' ')
Start-Sleep -Milliseconds 800
Probe 'after space'
'--- 本次新增日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
