# 排查：带 --trace 的实例 + 真实 cmd 窗口逐键输入
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

# 1) 停掉已安装实例，换成带追踪的开发版
Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --terminal-only --trace'
}
Start-Sleep -Milliseconds 1500
$dev = Get-Process -Id $res.ProcessId -ErrorAction SilentlyContinue
if (-not $dev) { '开发版实例启动失败'; exit 1 }
"trace 实例 pid: $($res.ProcessId)"

# 2) 起一个独立 cmd 窗口
$res2 = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'cmd.exe /k "title MC-IME-TRACE-TEST & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
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

$hwnd = Find-ByTitle 'MC-IME-TRACE-TEST'
$cls = New-Object System.Text.StringBuilder 256
[void][W.T]::GetClassNameW($hwnd, $cls, 256)
"cmd 窗口: $hwnd 类=$($cls.ToString())"

$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread, $myThread, $true)
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread, $myThread, $false)
Start-Sleep -Milliseconds 600

# 3) 人速逐键输入 nihao + 空格
foreach ($ch in 'n','i','h','a','o') {
  [System.Windows.Forms.SendKeys]::SendWait($ch)
  Start-Sleep -Milliseconds 260
}
Start-Sleep -Milliseconds 400
[System.Windows.Forms.SendKeys]::SendWait(' ')
Start-Sleep -Milliseconds 900

# 4) 所有候选窗（可能有多个进程各一个）
$script:wins = @()
$cb2 = [W.T+EnumProc]{
  param($h, $l)
  $s = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $s, 256)
  if ($s.ToString() -eq 'MCTypeEnglishCandidates') {
    [uint32]$p = 0
    [void][W.T]::GetWindowThreadProcessId($h, [ref]$p)
    $r = New-Object W.T+RECT
    [void][W.T]::GetWindowRect($h, [ref]$r)
    $script:wins += "  候选窗 hwnd=$h pid=$p visible=$([W.T]::IsWindowVisible($h)) rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"
  }
  return $true
}
[void][W.T]::EnumWindows($cb2, [IntPtr]::Zero)
'--- 全部候选窗 ---'
$script:wins

'--- 本次新增日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
