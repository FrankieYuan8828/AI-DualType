Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace W -Name A -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
"@
$ime = Start-Process "C:\MC-TypeEnglish\target\release\mcte.exe" -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1500
$form = New-Object System.Windows.Forms.Form
$form.Text='page test'; $form.Width=700; $form.Height=200; $form.StartPosition='CenterScreen'; $form.TopMost=$true
$tb = New-Object System.Windows.Forms.TextBox; $tb.Multiline=$true; $tb.Dock='Fill'
$tb.Font = New-Object System.Drawing.Font("Microsoft YaHei UI",16)
$form.Controls.Add($tb); $form.Show()
for($i=0;$i -lt 15;$i++){ $form.Activate(); [void]$tb.Focus(); [void][W.A]::SetForegroundWindow($form.Handle); Start-Sleep -Milliseconds 60; if([W.A]::GetForegroundWindow() -eq $form.Handle){break} }
[System.Windows.Forms.SendKeys]::SendWait("shi"); Start-Sleep -Milliseconds 700
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
function Shot($path){ $bmp=New-Object System.Drawing.Bitmap($b.Width,$b.Height); $g=[System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $bmp.Save($path,[System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose() }
Shot 'C:\MC-TypeEnglish\screenshots\ui-page1.png'
[System.Windows.Forms.SendKeys]::SendWait("{PGDN}"); Start-Sleep -Milliseconds 600
Shot 'C:\MC-TypeEnglish\screenshots\ui-page2.png'
"page2 textbox='$($tb.Text)'"
[System.Windows.Forms.SendKeys]::SendWait("{ESC}")
$form.Close(); Stop-Process -Id $ime.Id -Force -ErrorAction SilentlyContinue
'done'
