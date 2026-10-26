# 切到 MC-TypeEnglish 后截任务栏右下角，检查输入指示器图标
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Runtime.InteropServices -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
[ComImport, Guid("71C6E74C-0F28-11D8-A82A-00065B84435C"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface ITfInputProcessorProfileMgr {
    void ActivateProfile(uint t, ushort langid, ref Guid clsid, ref Guid prof, IntPtr hkl, uint flags);
}
[ComImport, Guid("33c53a50-f456-4884-b049-85fd643ecfed")]
public class TF_InputProcessorProfiles { }
public static class Sw {
    public static void Go(Guid c, Guid p, ushort l) {
        var m = (ITfInputProcessorProfileMgr)new TF_InputProcessorProfiles();
        m.ActivateProfile(1, l, ref c, ref p, IntPtr.Zero, 0);
    }
}
"@
$form = New-Object System.Windows.Forms.Form
$form.Text = "icon check"; $form.Width = 600; $form.Height = 160; $form.StartPosition='CenterScreen'; $form.TopMost=$true
$tb = New-Object System.Windows.Forms.TextBox; $tb.Dock='Fill'; $tb.Font = New-Object System.Drawing.Font("Microsoft YaHei UI",14)
$form.Controls.Add($tb); $form.Show(); $form.Activate(); $tb.Focus()
[Sw]::Go([Guid]'{7C3E0C11-5F6A-4E52-9B2C-A1D47B2E9F31}', [Guid]'{B3D2F0A4-6C81-4E17-9E45-2D8A9C6B4F72}', 0x0804)
$end=(Get-Date).AddMilliseconds(1500); while((Get-Date) -lt $end){[System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30}
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size)
New-Item -ItemType Directory -Force -Path C:\MC-TypeEnglish\screenshots | Out-Null
$bmp.Save('C:\MC-TypeEnglish\screenshots\icon-full.png',[System.Drawing.Imaging.ImageFormat]::Png)
# 裁右下角任务栏（宽度 60% 到 100%，高度底部 60px）
$rect = New-Object System.Drawing.Rectangle([int]($b.Width*0.45), $b.Height-70, [int]($b.Width*0.55), 70)
$crop = New-Object System.Drawing.Bitmap(($rect.Width*3), ($rect.Height*3))
$g2 = [System.Drawing.Graphics]::FromImage($crop)
$g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g2.DrawImage($bmp,(New-Object System.Drawing.Rectangle(0,0,$crop.Width,$crop.Height)),$rect,[System.Drawing.GraphicsUnit]::Pixel)
$crop.Save('C:\MC-TypeEnglish\screenshots\icon-taskbar-crop.png',[System.Drawing.Imaging.ImageFormat]::Png)
$g2.Dispose(); $crop.Dispose(); $g.Dispose(); $bmp.Dispose(); $form.Close()
"screen: $($b.Width)x$($b.Height)  crop saved"
