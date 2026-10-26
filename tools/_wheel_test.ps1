# 验证设置弹窗的鼠标滚轮：滚一格，高亮下移一行
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
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, UIntPtr w, IntPtr l);
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
  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc(); [void][W.T]::PrintWindow($hwnd, $dc, 1); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  return "${w}x${h}"
}

function ClickIn($hwnd, $clientX, $clientY) {
  $lp = [IntPtr](($clientY -shl 16) -bor ($clientX -band 0xFFFF))
  [void][W.T]::PostMessageW($hwnd, 0x0201, [UIntPtr]::Zero, $lp)
  Start-Sleep -Milliseconds 80
}

Get-Process mcte -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
$res = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = '"C:\MC-TypeEnglish\target\release\mcte.exe" --demo nihao'
  CurrentDirectory = 'C:\MC-TypeEnglish'
}
Start-Sleep -Milliseconds 1500

$cand = Find-ByClass 'MCTypeEnglishCandidates' $res.ProcessId
ClickIn $cand 447 18      # 右上角「设置」
Start-Sleep -Milliseconds 400
$panel = Find-ByClass 'MCTypeEnglishSettings' $null
"语言弹窗: $panel"
if ($panel -eq [IntPtr]::Zero) { Stop-Process -Id $res.ProcessId -Force; exit 1 }

Shot $panel 'C:\MC-TypeEnglish\screenshots\settings-wheel-before.png' | Out-Null

# WM_MOUSEWHEEL：HIWORD 存放有符号 delta，向下滚一格 = -120（0xFF88 in HIWORD）
$wheelDown = [UIntPtr]::new([uint64]4287102976)
[void][W.T]::PostMessageW($panel, 0x020A, $wheelDown, [IntPtr]::Zero)
Start-Sleep -Milliseconds 300
[void][W.T]::PostMessageW($panel, 0x020A, $wheelDown, [IntPtr]::Zero)
Start-Sleep -Milliseconds 400

Shot $panel 'C:\MC-TypeEnglish\screenshots\settings-wheel-after.png' | Out-Null
'saved settings-wheel-before.png / settings-wheel-after.png'
Stop-Process -Id $res.ProcessId -Force -ErrorAction SilentlyContinue
