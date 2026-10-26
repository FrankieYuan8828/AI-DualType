Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@
$p = Start-Process 'C:\MC-TypeEnglish\target\release\mcte.exe' -ArgumentList '--demo','nihao' -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1200
$script:found = [IntPtr]::Zero
$cb = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
    $r = New-Object W.T+RECT
    [void][W.T]::GetWindowRect($h, [ref]$r)
    Write-Output ("cand hwnd={0} visible={1} rect={2},{3},{4},{5}" -f $h, [W.T]::IsWindowVisible($h), $r.Left, $r.Top, $r.Right, $r.Bottom)
    if ($script:found -eq [IntPtr]::Zero) { $script:found = $h }
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
"屏幕: $($b.Width)x$($b.Height) at $($b.X),$($b.Y)"
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save('C:\MC-TypeEnglish\screenshots\ui-full.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
'saved ui-full.png'
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
