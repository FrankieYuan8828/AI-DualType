# 列出并清理所有残留的候选窗（MCTypeEnglishCandidates）
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

function Get-CandidateWindows {
  $script:list = @()
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $c = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($h, $c, 256)
    if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
      [uint32]$p = 0
      [void][W.T]::GetWindowThreadProcessId($h, [ref]$p)
      $r = New-Object W.T+RECT
      [void][W.T]::GetWindowRect($h, [ref]$r)
      $t = New-Object System.Text.StringBuilder 256
      [void][W.T]::GetWindowTextW($h, $t, 256)
      $proc = (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
      $script:list += [pscustomobject]@{
        Hwnd = $h; Pid = $p; Proc = $proc; Visible = [W.T]::IsWindowVisible($h)
        Rect = "$($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"; Title = $t.ToString()
      }
    }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:list
}

'--- 清理前 ---'
$before = Get-CandidateWindows
$before | Format-Table -AutoSize

$hidden = 0
foreach ($w in $before) {
  if ($w.Visible) { [void][W.T]::ShowWindow([IntPtr]$w.Hwnd, 0); $hidden++ }
}
Start-Sleep -Milliseconds 300
"已隐藏 $hidden 个可见候选窗"

'--- 清理后 ---'
Get-CandidateWindows | Format-Table -AutoSize
