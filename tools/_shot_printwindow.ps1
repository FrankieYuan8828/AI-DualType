Add-Type -AssemblyName System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@
$p = Start-Process 'C:\MC-TypeEnglish\target\release\mcte.exe' -ArgumentList '--demo','nihao' -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1500

$script:found = [IntPtr]::Zero
$script:info = ''
$cb = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
    $r = New-Object W.T+RECT
    [void][W.T]::GetWindowRect($h, [ref]$r)
    $script:found = $h
    $script:info = "hwnd=$h visible=$([W.T]::IsWindowVisible($h)) rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom) size=$($r.Right-$r.Left)x$($r.Bottom-$r.Top)"
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
$script:info
if ($script:found -eq [IntPtr]::Zero) { Stop-Process -Id $p.Id -Force; exit 1 }

$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($script:found, [ref]$r)
$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
$ok = [W.T]::PrintWindow($script:found, $dc, 2)
$g.ReleaseHdc($dc)
$g.Dispose()
$bmp.Save('C:\MC-TypeEnglish\screenshots\ui-printwindow.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"PrintWindow=$ok saved ${w}x${h}"
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
