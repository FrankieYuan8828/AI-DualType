Add-Type -AssemblyName System.Drawing
Add-Type -Namespace D -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][D.P]::SetProcessDPIAware()
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

$p = Start-Process 'C:\MC-TypeEnglish\target\release\mcte.exe' -ArgumentList '--demo','nihao' -PassThru
Start-Sleep -Milliseconds 1200

$script:hit = [IntPtr]::Zero
$cb = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
    [uint32]$owner = 0
    [void][W.T]::GetWindowThreadProcessId($h, [ref]$owner)
    if ($owner -eq $p.Id) { $script:hit = $h; return $false }
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
$hwnd = $script:hit
if ($hwnd -ne [IntPtr]::Zero) {
  $r = New-Object W.T+RECT; [void][W.T]::GetWindowRect($hwnd, [ref]$r)
  $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save('C:\MC-TypeEnglish\screenshots\test-demo-nihao.png', [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "Saved test-demo-nihao.png ${w}x${h}"
} else {
  "Window not found"
}
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
