Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name A -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@
$ime = Start-Process "C:\MC-TypeEnglish\target\release\mcte.exe" -ArgumentList '--demo','nihao' -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1600
[int]$target = $ime.Id
$script:h = [IntPtr]::Zero
$script:rect = New-Object W.A+RECT
$cb = [W.A+EnumProc]{
  param($hwnd, $l)
  [uint32]$winPid = 0
  [void][W.A]::GetWindowThreadProcessId($hwnd, [ref]$winPid)
  if ($winPid -eq $target) {
    $sb = New-Object System.Text.StringBuilder 256
    [void][W.A]::GetClassNameW($hwnd, $sb, 256)
    if ($sb.ToString() -eq 'MCTypeEnglishCandidates') { $script:h = $hwnd; return $false }
  }
  return $true
}
[void][W.A]::EnumWindows($cb, [IntPtr]::Zero)
"window: $script:h"
if ($script:h -ne [IntPtr]::Zero) {
  [void][W.A]::GetWindowRect($script:h, [ref]$script:rect)
  $r = $script:rect
  $w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
  "rect: $($r.Left),$($r.Top) ${w}x${hh}"
  # 卡片区：y = top + 36 .. +90；上层中心 = top+36+12，下层中心 = top+36+6+24+9
  $cx = $r.Left + 40
  $top_cn = $r.Top + 36 + 12
  $top_en = $r.Top + 36 + 6 + 24 + 10
  foreach ($pt in @(@($cx,$top_cn,'中文层'), @($cx,$top_en,'英文层'))) {
    [void][W.A]::SetCursorPos($pt[0], $pt[1])
    Start-Sleep -Milliseconds 120
    [W.A]::mouse_event(0x0002,0,0,0,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [W.A]::mouse_event(0x0004,0,0,0,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 300
    "clicked $($pt[2]) at $($pt[0]),$($pt[1])"
  }
}
Start-Sleep -Milliseconds 400
Stop-Process -Id $ime.Id -Force -ErrorAction SilentlyContinue
