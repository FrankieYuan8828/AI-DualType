Add-Type -AssemblyName System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

# 用 WMI 启动，避开 Start-Process -WindowStyle Hidden 对首次 ShowWindow 的压制
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --demo nihao'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
$demoPid = $res.ProcessId
"demo pid: $demoPid"
Start-Sleep -Milliseconds 1600

function Find-Candidate($pidFilter) {
  $script:hit = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $c = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($h, $c, 256)
    if ($c.ToString() -eq 'MCTypeEnglishCandidates') {
      [uint32]$owner = 0
      [void][W.T]::GetWindowThreadProcessId($h, [ref]$owner)
      if ($owner -eq $pidFilter) { $script:hit = $h; return $false }
    }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:hit
}

$hwnd = Find-Candidate $demoPid
"demo 自己的候选窗: $hwnd"
if ($hwnd -eq [IntPtr]::Zero) { Stop-Process -Id $demoPid -Force -ErrorAction SilentlyContinue; exit 1 }

$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($hwnd, [ref]$r)
"visible=$([W.T]::IsWindowVisible($hwnd)) rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom) size=$($r.Right-$r.Left)x$($r.Bottom-$r.Top)"

$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
$ok = [W.T]::PrintWindow($hwnd, $dc, 2)
$g.ReleaseHdc($dc); $g.Dispose()
$bmp.Save('C:\MC-TypeEnglish\screenshots\ui-card-multi.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"PrintWindow=$ok saved ${w}x${h}"
Stop-Process -Id $demoPid -Force -ErrorAction SilentlyContinue
