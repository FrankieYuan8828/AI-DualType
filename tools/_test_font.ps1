Add-Type -AssemblyName System.Drawing

foreach ($sz in @(6.5, 7, 7.5, 8)) {
    $bmp = New-Object System.Drawing.Bitmap(16, 16, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::FromArgb(255, 9, 105, 218))
    $font = New-Object System.Drawing.Font('Arial', $sz, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $fmt.FormatFlags = [System.Drawing.StringFormatFlags]::NoWrap
    $rect = New-Object System.Drawing.RectangleF(0, 0, 16, 16)
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
    $g.DrawString('MC', $font, $white, $rect, $fmt)
    $bmp.Save("screenshots\test-mc-$sz.png", [System.Drawing.Imaging.ImageFormat]::Png)
    $measured = $g.MeasureString('MC', $font, [System.Drawing.PointF]::Empty, $fmt)
    "$sz px: width=$($measured.Width)"
}
