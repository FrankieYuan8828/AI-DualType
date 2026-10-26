Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win -Name W -MemberDefinition @"
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string name);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@
$p = Start-Process "C:\MC-TypeEnglish\target\release\mcte.exe" -ArgumentList '--demo','shi' -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1500
$h = [Win.W]::FindWindowW("MCTypeEnglishCandidates", $null)
"window handle: $h"
$r = New-Object Win.W+RECT
[void][Win.W]::GetWindowRect($h, [ref]$r)
"rect: $($r.Left),$($r.Top) - $($r.Right),$($r.Bottom)  =>  $($r.Right-$r.Left)x$($r.Bottom-$r.Top)"
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size)
$pad = 12
$x = [Math]::Max(0, $r.Left-$pad); $y = [Math]::Max(0, $r.Top-$pad)
$w = ($r.Right-$r.Left) + 2*$pad; $hh = ($r.Bottom-$r.Top) + 2*$pad
$crop = New-Object System.Drawing.Bitmap($w, $hh)
$g2 = [System.Drawing.Graphics]::FromImage($crop)
$g2.DrawImage($bmp, (New-Object System.Drawing.Rectangle(0,0,$w,$hh)), (New-Object System.Drawing.Rectangle($x,$y,$w,$hh)), [System.Drawing.GraphicsUnit]::Pixel)
$crop.Save('C:\MC-TypeEnglish\screenshots\ui-card-zoom.png',[System.Drawing.Imaging.ImageFormat]::Png)
$g2.Dispose(); $crop.Dispose(); $g.Dispose(); $bmp.Dispose()
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
"saved crop ${w}x${hh}"
