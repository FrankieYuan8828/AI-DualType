# 截取候选窗：按窗口类名枚举（FindWindow 在这台机器上偶发找不到）
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
Start-Sleep -Milliseconds 1400

$script:found = [IntPtr]::Zero
$cb = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates' -and [W.T]::IsWindowVisible($h)) { $script:found = $h; return $false }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
$h = $script:found
if ($h -eq [IntPtr]::Zero) { '未找到候选窗'; Stop-Process -Id $p.Id -Force; exit 1 }

$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($h, [ref]$r)
"候选窗 rect: $($r.Left),$($r.Top),$($r.Right),$($r.Bottom) => $($r.Right-$r.Left)x$($r.Bottom-$r.Top)"

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)

$pad = 10
$x = [Math]::Max(0, $r.Left - $pad); $y = [Math]::Max(0, $r.Top - $pad)
$w = ($r.Right - $r.Left) + 2*$pad; $hh = ($r.Bottom - $r.Top) + 2*$pad
$crop = New-Object System.Drawing.Bitmap($w, $hh)
$g2 = [System.Drawing.Graphics]::FromImage($crop)
$g2.DrawImage($bmp, (New-Object System.Drawing.Rectangle(0,0,$w,$hh)), (New-Object System.Drawing.Rectangle($x,$y,$w,$hh)), [System.Drawing.GraphicsUnit]::Pixel)
$crop.Save('C:\MC-TypeEnglish\screenshots\ui-card-multi.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g2.Dispose(); $crop.Dispose(); $g.Dispose(); $bmp.Dispose()
"saved ui-card-multi.png (${w}x${hh})"
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
