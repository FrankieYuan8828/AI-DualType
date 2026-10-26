# 验证：常驻实例里点设置 → 选德语 → 配置落盘 + 候选立刻多出德语行
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
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, UIntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId2(IntPtr h);
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

function Find-ByTitle([string]$needle) {
  $script:found = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $s = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetWindowTextW($h, $s, 256)
    if ($s.ToString() -like "*$needle*") { $script:found = $h; return $false }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:found
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
  return "saved $path (${w}x${h})"
}

function ClickIn($hwnd, $clientX, $clientY) {
  $lp = [IntPtr](($clientY -shl 16) -bor ($clientX -band 0xFFFF))
  [void][W.T]::PostMessageW($hwnd, 0x0201, [UIntPtr]::Zero, $lp)
  Start-Sleep -Milliseconds 80
  [void][W.T]::PostMessageW($hwnd, 0x0202, [UIntPtr]::Zero, $lp)
}

$cfgPath = "$env:APPDATA\MC-TypeEnglish\config.json"
# 起点：不额外语言
$cfg = Get-Content $cfgPath -Raw -Encoding UTF8 | ConvertFrom-Json
$cfg.extra_language = $null
[IO.File]::WriteAllText($cfgPath, ($cfg | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding($false)))

Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$app = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --terminal-only'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1500
"mcte pid: $($app.ProcessId)"

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-SETTINGS & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1800

$hwnd = Find-ByTitle 'MC-IME-SETTINGS'
# 抢焦点
[System.Windows.Forms.SendKeys]::SendWait('%')
Start-Sleep -Milliseconds 150
$fg = [W.T]::GetForegroundWindow()
[uint32]$fgPid = 0
$fgThread = [W.T]::GetWindowThreadProcessId($fg, [ref]$fgPid)
$myThread = [System.Runtime.InteropServices.Marshal]::GetCurrentThreadId()
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 600
"前台就是 cmd: $([W.T]::GetForegroundWindow() -eq $hwnd)"

[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 200
foreach ($ch in 'nihao'.ToCharArray()) { [System.Windows.Forms.SendKeys]::SendWait([string]$ch); Start-Sleep -Milliseconds 120 }
Start-Sleep -Milliseconds 400

$cand = Find-ByClass 'MCTypeEnglishCandidates' $app.ProcessId
"候选窗: $cand"
Shot $cand 'C:\MC-TypeEnglish\screenshots\ui-before-extra.png'

$r = New-Object W.T+RECT
[void][W.T]::GetWindowRect($cand, [ref]$r)
ClickIn $cand (($r.Right - $r.Left) - 26) 18
Start-Sleep -Milliseconds 500
$panel = Find-ByClass 'MCTypeEnglishSettings' $null
"语言弹窗: $panel"
if ($panel -eq [IntPtr]::Zero) { '没弹出设置窗'; Stop-Process -Id $app.ProcessId -Force; exit 1 }

Shot $panel 'C:\MC-TypeEnglish\screenshots\ui-settings-live.png'
ClickIn $panel 60 (10 + 26 + 1*30 + 14)   # 德语
Start-Sleep -Milliseconds 1200

'--- 配置 ---'
Get-Content $cfgPath -Raw -Encoding UTF8
'--- 选完后的候选窗 ---'
$cand2 = Find-ByClass 'MCTypeEnglishCandidates' $app.ProcessId
Shot $cand2 'C:\MC-TypeEnglish\screenshots\ui-after-extra.png'

Stop-Process -Id $app.ProcessId -Force -ErrorAction SilentlyContinue
