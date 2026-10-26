# 直接验证"正在运行的那个实例"（可能是提权启动的）
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace D -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][D.P]::SetProcessDPIAware()
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
"@

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

function Find-Cand {
  $script:hit = [IntPtr]::Zero
  $cb = [W.T+EnumProc]{
    param($h, $l)
    $c = New-Object System.Text.StringBuilder 256
    [void][W.T]::GetClassNameW($h, $c, 256)
    if ($c.ToString() -eq 'MCTypeEnglishCandidates' -and [W.T]::IsWindowVisible($h)) { $script:hit = $h; return $false }
    return $true
  }
  [void][W.T]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:hit
}

function Shot($hwnd, $path) {
  $r = New-Object W.T+RECT
  [void][W.T]::GetWindowRect($hwnd, [ref]$r)
  $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  if ($w -le 0) { return 'bad rect' }
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  return "${w}x${h}"
}

$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = 'conhost.exe cmd.exe /k "title MC-IME-LIVE & echo ready"'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1800
$hwnd = Find-ByTitle 'MC-IME-LIVE'
if ($hwnd -eq [IntPtr]::Zero) { '未找到测试窗口'; exit 1 }

[System.Windows.Forms.SendKeys]::SendWait('%')
Start-Sleep -Milliseconds 150
[void][W.T]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 600
"前台就是 cmd: $([W.T]::GetForegroundWindow() -eq $hwnd)"

[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 200
foreach ($ch in 'nihao'.ToCharArray()) { [System.Windows.Forms.SendKeys]::SendWait([string]$ch); Start-Sleep -Milliseconds 130 }
Start-Sleep -Milliseconds 3500

$cand = Find-Cand
if ($cand -eq [IntPtr]::Zero) { '候选窗不可见（输入法可能没在跑）'; exit 1 }
"候选窗 ${cand}"

# 提权进程的窗口内容普通权限读不到（UIPI），改抓整屏
[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 200
foreach ($ch in 'nihao'.ToCharArray()) { [System.Windows.Forms.SendKeys]::SendWait([string]$ch); Start-Sleep -Milliseconds 130 }
Start-Sleep -Milliseconds 2500

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save('C:\MC-TypeEnglish\screenshots\installed-blue-screen.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"saved installed-blue-screen.png (${($b.Width)}x${($b.Height)})"
[System.Windows.Forms.SendKeys]::SendWait('{ESC}')

