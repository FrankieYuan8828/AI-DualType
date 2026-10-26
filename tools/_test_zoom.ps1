Add-Type -AssemblyName System.Drawing

function Render-Zoom($size, $fontName, $fontPx, $outFile) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    $radius = [Math]::Max(2, [int]($size * 0.22))
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $w = $size; $h = $size
    $path.AddArc(0, 0, $d, $d, 180, 90)
    $path.AddArc($w - $d, 0, $d, $d, 270, 90)
    $path.AddArc($w - $d, $h - $d, $d, $d, 0, 90)
    $path.AddArc(0, $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 9, 105, 218))
    $g.FillPath($brush, $path)

    $font = New-Object System.Drawing.Font($fontName, $fontPx, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $fmt.FormatFlags = [System.Drawing.StringFormatFlags]::NoWrap
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
    $rect = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
    $rect.Y = -($size * 0.03)
    $g.DrawString("MC", $font, $white, $rect, $fmt)

    # Zoom 8x for visual check
    $zoom = New-Object System.Drawing.Bitmap(($size * 8), ($size * 8))
    $gz = [System.Drawing.Graphics]::FromImage($zoom)
    $gz.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $gz.DrawImage($bmp, 0, 0, ($size * 8), ($size * 8))
    $zoom.Save($outFile, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Save($outFile.Replace('-zoom.png', '.png'), [System.Drawing.Imaging.ImageFormat]::Png)
}

Render-Zoom 16 'Segoe UI' 7.5 'screenshots\mc-16-segoe-7.5-zoom.png'
Render-Zoom 16 'Arial' 7.5 'screenshots\mc-16-arial-7.5-zoom.png'
Render-Zoom 16 'Segoe UI' 7.0 'screenshots\mc-16-segoe-7.0-zoom.png'
Render-Zoom 20 'Segoe UI' 9.5 'screenshots\mc-20-segoe-9.5-zoom.png'
"Rendered zoom images."
