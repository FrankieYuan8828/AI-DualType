# 诊断：现有 cmd / powershell 进程的窗口类名与是否提权
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
"@
Add-Type -Namespace W -Name Sec -MemberDefinition @"
[DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
[DllImport("kernel32.dll", SetLastError=true)] public static extern bool CloseHandle(IntPtr h);
[DllImport("advapi32.dll", SetLastError=true)] public static extern bool OpenProcessToken(IntPtr p, uint access, out IntPtr token);
[DllImport("advapi32.dll", SetLastError=true)] public static extern bool GetTokenInformation(IntPtr token, int cls, out uint info, uint len, out uint ret);
"@

$targets = Get-Process | Where-Object { $_.ProcessName -match '^(cmd|powershell|pwsh|WindowsTerminal|OpenConsole|conhost)$' }
$pids = $targets.Id

$windows = @()
$cb = [W.T+EnumProc]{
  param($h, $l)
  [uint32]$p = 0
  [void][W.T]::GetWindowThreadProcessId($h, [ref]$p)
  if ($pids -contains $p) {
    $c = New-Object System.Text.StringBuilder 256
    $t = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($h, $c, 256)
    [void][W.T]::GetWindowTextW($h, $t, 256)
    $script:windows += [pscustomobject]@{
      Pid = $p; Class = $c.ToString(); Title = $t.ToString(); Visible = [W.T]::IsWindowVisible($h)
    }
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)

'--- 相关进程与窗口 ---'
foreach ($proc in $targets) {
  $w = $windows | Where-Object { $_.Pid -eq $proc.Id }
  $cls = if ($w) { ($w | ForEach-Object { $_.Class }) -join ', ' } else { '(无顶层窗口)' }
  # 提权检测：TokenElevation(20) => 1 表示已提升
  $elev = 'n/a'
  try {
    $tok = [IntPtr]::Zero
    if ([W.Sec]::OpenProcessToken($proc.Handle, 0x0008, [ref]$tok)) {
      [uint32]$info = 0; [uint32]$ret = 0
      if ([W.Sec]::GetTokenInformation($tok, 20, [ref]$info, 4, [ref]$ret)) { $elev = if ($info -eq 1) { 'ELEVATED' } else { 'normal' } }
      [void][W.Sec]::CloseHandle($tok)
    }
  } catch { $elev = "err: $_" }
  "{0,-6} {1,-16} {2,-9} {3}" -f $proc.Id, $proc.ProcessName, $elev, $cls
}
