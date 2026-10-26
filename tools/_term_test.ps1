Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace W -Name T -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder b, int n);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public delegate bool EnumProc(IntPtr h, IntPtr l);
"@
function Get-Terminals {
  $list = New-Object System.Collections.ArrayList
  $cb=[W.T+EnumProc]{
    param($h,$l)
    if ([W.T]::IsWindowVisible($h)) {
      $sb=New-Object System.Text.StringBuilder 256
      [void][W.T]::GetClassNameW($h,$sb,256)
      if ($sb.ToString() -eq 'CASCADIA_HOSTING_WINDOW_CLASS') { [void]$list.Add($h) }
    }
    return $true
  }
  [void][W.T]::EnumWindows($cb,[IntPtr]::Zero)
  return $list
}
function Shot($path) {
  $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp=New-Object System.Drawing.Bitmap($b.Width,$b.Height)
  $g=[System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size)
  $bmp.Save($path,[System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
}
$before = Get-Terminals
"现有终端窗口: $($before.Count)"
$wt = Start-Process wt.exe -ArgumentList 'cmd','/k','echo MC-IME-TEST' -PassThru
$new = [IntPtr]::Zero
for ($i=0; $i -lt 20; $i++) {
  Start-Sleep -Milliseconds 500
  $now = Get-Terminals
  $diff = $now | Where-Object { $before -notcontains $_ }
  if ($diff) { $new = $diff[0]; break }
}
"新窗口: $new"; $sb2=New-Object System.Text.StringBuilder 256; [void][W.T]::GetClassNameW($new,$sb2,256); "窗口类: $($sb2.ToString())"
if ($new -eq [IntPtr]::Zero) { '未检测到新终端窗口'; exit 1 }
$fg=[W.T]::GetForegroundWindow()
[uint32]$fgPid=0; $fgThread=[W.T]::GetWindowThreadProcessId($fg,[ref]$fgPid)
$myThread=[W.T]::GetCurrentThreadId()
[void][W.T]::AttachThreadInput($fgThread,$myThread,$true)
[void][W.T]::SetForegroundWindow($new)
Start-Sleep -Milliseconds 400
[void][W.T]::AttachThreadInput($fgThread,$myThread,$false)
Start-Sleep -Milliseconds 600
"foreground==new: $([W.T]::GetForegroundWindow() -eq $new)"
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Start-Sleep -Milliseconds 900
Shot 'C:\MC-TypeEnglish\screenshots\term-test-1.png'
[System.Windows.Forms.SendKeys]::SendWait("1")
Start-Sleep -Milliseconds 250
[System.Windows.Forms.SendKeys]::SendWait(" ")
Start-Sleep -Milliseconds 900
Shot 'C:\MC-TypeEnglish\screenshots\term-test-2.png'
'--- 日志（应有 commit）---'
Get-Content (Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log') -Tail 5 -Encoding UTF8
Start-Sleep -Milliseconds 300
