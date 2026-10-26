# 验证：实时输入时，默认蓝底必须只在中文那一行（译文到达前后各截一张）
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
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
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
  if ($w -le 0) { return }
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
}

Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$app = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\Program Files\MC-TypeEnglish\mcte.exe" --terminal-only'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1500

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-BLUE & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1800
$hwnd = Find-ByTitle 'MC-IME-BLUE'

[System.Windows.Forms.SendKeys]::SendWait('%')
Start-Sleep -Milliseconds 150
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 600
"前台就是 cmd: $([W.T]::GetForegroundWindow() -eq $hwnd)"

[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 200
foreach ($ch in 'nihao'.ToCharArray()) { [System.Windows.Forms.SendKeys]::SendWait([string]$ch); Start-Sleep -Milliseconds 130 }
Start-Sleep -Milliseconds 300

$cand = Find-ByClass 'MCTypeEnglishCandidates' $app.ProcessId
Shot $cand 'C:\MC-TypeEnglish\screenshots\installed-blue-1.png'
'第一张已存（刚打完拼音）'
Start-Sleep -Milliseconds 4000
$cand2 = Find-ByClass 'MCTypeEnglishCandidates' $app.ProcessId
Shot $cand2 'C:\MC-TypeEnglish\screenshots\installed-blue-2.png'
'第二张已存（翻译返回之后）'
# 保留运行中的实例

