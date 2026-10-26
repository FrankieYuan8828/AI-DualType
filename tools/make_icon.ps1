# 生成 MC-TypeEnglish 的输入法图标（多尺寸 .ico，PNG 帧）。
#
# 为什么不用第三方库：System.Drawing 就能画圆角方块 + 文字，然后手工拼 ICO 容器，
# 不引入任何依赖。
#
# 用法： powershell -ExecutionPolicy Bypass -File tools\make_icon.ps1

param([string]$Out = "data\mcte.ico")

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$outPath = Join-Path $root $Out
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outPath) | Out-Null

$sizes = @(16, 20, 24, 32, 40, 48, 64, 128, 256)
$frames = @()

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    # 圆角方块：GitHub 蓝 #0969DA
    $radius = if ($size -le 16) { 2 } elseif ($size -le 24) { 3 } else { [Math]::Max(3, [int]($size * 0.22)) }
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

    # 「MC」两字：字号在小尺寸下精细对齐，必须加 NoWrap 禁止折行（否则 16x16 会把 M 和 C 拆成上下两行）
    $fontPx = switch ($size) {
        16 { 7.5 }
        20 { 9.5 }
        24 { 11.5 }
        32 { 15.0 }
        default { [int]($size * 0.46) }
    }
    $font = New-Object System.Drawing.Font("Segoe UI", $fontPx, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $fmt.FormatFlags = [System.Drawing.StringFormatFlags]::NoWrap
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
    $rect = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
    # 视觉微调：文字重心略偏上一点更像图标
    $rect.Y = -($size * 0.03)
    $g.DrawString("MC", $font, $white, $rect, $fmt)

    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $frames += , @{ Size = $size; Data = $ms.ToArray() }

    $ms.Dispose(); $white.Dispose(); $font.Dispose(); $brush.Dispose(); $path.Dispose(); $g.Dispose(); $bmp.Dispose()
}

# ---- 拼 ICO 容器：ICONDIR + N 个 ICONDIRENTRY + PNG 数据 ----
$stream = New-Object System.IO.MemoryStream
$writer = New-Object System.IO.BinaryWriter($stream)

$writer.Write([UInt16]0)                 # reserved
$writer.Write([UInt16]1)                 # type = icon
$writer.Write([UInt16]$frames.Count)     # count

$offset = 6 + 16 * $frames.Count
foreach ($frame in $frames) {
    $dim = if ($frame.Size -ge 256) { 0 } else { $frame.Size }
    $writer.Write([Byte]$dim)            # width
    $writer.Write([Byte]$dim)            # height
    $writer.Write([Byte]0)               # palette
    $writer.Write([Byte]0)               # reserved
    $writer.Write([UInt16]1)             # planes
    $writer.Write([UInt16]32)            # bpp
    $writer.Write([UInt32]$frame.Data.Length)
    $writer.Write([UInt32]$offset)
    $offset += $frame.Data.Length
}
foreach ($frame in $frames) { $writer.Write($frame.Data) }
$writer.Flush()
[IO.File]::WriteAllBytes($outPath, $stream.ToArray())
$writer.Dispose(); $stream.Dispose()

Write-Host ("生成 {0}：{1} 个尺寸，{2} 字节" -f $Out, $frames.Count, (Get-Item $outPath).Length)
