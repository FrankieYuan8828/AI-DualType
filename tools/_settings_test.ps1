# 验证：候选窗右上角「设置」→ 语言弹窗 → 选中德语
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace D -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][D.P]::SetProcessDPIAware()
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, UIntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

function Find-ByClass([string]$cls, $pidFilter) {
  $script:hit = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $c = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($h, $c, 256)
    if ($c.ToString() -eq $cls -and [W.T]::IsWindowVisible($h)) {
      if ($null -eq $pidFilter) { $script:hit = $h; return $false }
      [uint32]$owner = 0
      [void][W.T]::GetWindowThreadProcessId($h, [ref]$owner)
      if ($owner -eq $pidFilter) { $script:hit = $h; return $false }
    }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:hit
}

function Shot($hwnd, $path) {
  $r = New-Object W.T+RECT
  [void][W.T]::GetWindowRect($hwnd, [ref]$r)
  $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  if ($w -le 0 -or $h -le 0) { return "bad rect" }
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  return "saved $path (${w}x${h}) at $($r.Left),$($r.Top)"
}

function Click($x, $y) {
  [void][W.T]::SetCursorPos($x, $y)
  Start-Sleep -Milliseconds 120
  [W.T]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)  # LEFTDOWN
  Start-Sleep -Milliseconds 60
  [W.T]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)  # LEFTUP
}

# 直接投递 WM_LBUTTONDOWN：后台窗口被遮挡时真实鼠标点不到，这样测的是窗口自身逻辑
function ClickIn($hwnd, $clientX, $clientY) {
  $lp = [IntPtr](($clientY -shl 16) -bor ($clientX -band 0xFFFF))
  [void][W.T]::PostMessageW($hwnd, 0x0201, [UIntPtr]::Zero, $lp)
  Start-Sleep -Milliseconds 80
  [void][W.T]::PostMessageW($hwnd, 0x0202, [UIntPtr]::Zero, $lp)
}

$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Content $log -Encoding UTF8).Count

Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --demo nihao'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
$demoPid = $res.ProcessId
Start-Sleep -Milliseconds 1600

$cand = Find-ByClass 'MCTypeEnglishCandidates' $demoPid
"候选窗: $cand"
Shot $cand 'C:\MC-TypeEnglish\screenshots\ui-gear.png'

$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($cand, [ref]$r)
# 设置按钮在右上角：客户端坐标 (宽-26, 18)
$clientX = ($r.Right - $r.Left) - 26
"点击设置按钮 client=($clientX,18)"
ClickIn $cand $clientX 18
Start-Sleep -Milliseconds 500

$panel = Find-ByClass 'MCTypeEnglishSettings' $null
"语言弹窗: $panel"
if ($panel -ne [IntPtr]::Zero) {
  Shot $panel 'C:\MC-TypeEnglish\screenshots\ui-settings.png'
  # 行：0=不额外语言，1=德语；每行 30px，标题 26px，内边距 10px
  ClickIn $panel 60 (10 + 26 + 1*30 + 14)
  Start-Sleep -Milliseconds 500
}

'--- 日志 ---'
Get-Content $log -Encoding UTF8 | Select-Object -Skip $before
Stop-Process -Id $demoPid -Force -ErrorAction SilentlyContinue
