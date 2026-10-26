Add-Type -AssemblyName System.Windows.Forms
Add-Type -Namespace W -Name Z -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
"@
$log = Join-Path $env:APPDATA 'MC-TypeEnglish\mcte.log'
$before = (Get-Item $log).Length
$form = New-Object System.Windows.Forms.Form
$form.Text='non-terminal test'; $form.Width=600; $form.Height=180; $form.TopMost=$true; $form.StartPosition='CenterScreen'
$tb = New-Object System.Windows.Forms.TextBox; $tb.Multiline=$true; $tb.Dock='Fill'
$form.Controls.Add($tb); $form.Show()
for($i=0;$i -lt 20;$i++){ [void]$form.Activate(); [void]$tb.Focus(); [void][W.Z]::SetForegroundWindow($form.Handle); Start-Sleep -Milliseconds 80; if([W.Z]::GetForegroundWindow() -eq $form.Handle){break} }
"前台==表单: $([W.Z]::GetForegroundWindow() -eq $form.Handle)  (窗口类不是终端)"
[System.Windows.Forms.SendKeys]::SendWait("nihao")
Start-Sleep -Milliseconds 800
"文本框内容: '$($tb.Text)'  （应为 nihao，说明钩子没接管）"
$fs=[IO.File]::Open($log,'Open','Read','ReadWrite'); $fs.Seek($before,'Begin')|Out-Null
$buf=New-Object byte[] ($fs.Length-$before); [void]$fs.Read($buf,0,$buf.Length); $fs.Dispose()
$new = [Text.Encoding]::UTF8.GetString($buf)
"本次新增日志: $(if($new.Trim()){$new.Trim()}else{'（无，正确：非终端窗口没有 commit）'})"
$form.Close()
