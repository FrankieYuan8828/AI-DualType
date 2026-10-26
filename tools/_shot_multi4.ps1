Add-Type -AssemblyName System.Drawing
Add-Type -Namespace D -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
# 截图脚本必须声明 DPI 感知，否则坐标会被系统按缩放比虚拟化，截到空白
[void][D.P]::SetProcessDPIAware()
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@
$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --demo nihao --trace'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
$demoPid = $res.ProcessId
Start-Sleep -Milliseconds 1500

Get-Content $log -Encoding UTF8 | Select-Object -Skip $before | Where-Object { $_ -match 'ui:|demo:' }

$script:hit = [IntPtr]::Zero
$cb = [W.T+EnumProc]{
  param($h, $l)
  $c = New-Object System.Text.StringBuilder 256
  [void][W.T]::GetClassNameW($h, $c, 256)
  if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
    [uint32]$owner = 0
    [void][W.T]::GetWindowThreadProcessId($h, [ref]$owner)
    if ($owner -eq $demoPid) { $script:hit = $h; return $false }
  }
  return $true
}
[void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
$hwnd = $script:hit
$r = New-Object W.T+RECT; [void][W.T]::GetWindowRect($hwnd, [ref]$r)
$cr = New-Object W.T+RECT; [void][W.T]::GetClientRect($hwnd, [ref]$cr)
"window=$($r.Right-$r.Left)x$($r.Bottom-$r.Top)  client=$($cr.Right)x$($cr.Bottom)"

$w = $cr.Right; $h = $cr.Bottom
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc(); $ok = [W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
$bmp.Save('C:\MC-TypeEnglish\screenshots\ui-card-multi.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"PrintWindow=$ok saved ${w}x${h}"
Stop-Process -Id $demoPid -Force -ErrorAction SilentlyContinue
