# Render every slide of a .pptx to PNG with PowerPoint itself (true fonts), and optionally export a PDF.
# Usage: render_pptx.ps1 -Pptx <path> -PngDir <dir> [-Pdf] [-Width 1600]
param([Parameter(Mandatory)][string]$Pptx, [Parameter(Mandatory)][string]$PngDir, [switch]$Pdf, [int]$Width = 1600)
$ErrorActionPreference = "Stop"
$Pptx = (Resolve-Path $Pptx).Path
New-Item -ItemType Directory -Force $PngDir | Out-Null
Get-ChildItem $PngDir -Filter "slide-*.png" | Remove-Item
$app = New-Object -ComObject PowerPoint.Application
try {
    $p = $app.Presentations.Open($Pptx, $true, $false, $false)   # read-only, untitled no, window no
    $h = [int]($Width * $p.PageSetup.SlideHeight / $p.PageSetup.SlideWidth)
    for ($i = 1; $i -le $p.Slides.Count; $i++) {
        $p.Slides.Item($i).Export((Join-Path (Resolve-Path $PngDir) ("slide-{0:D2}.png" -f $i)), "PNG", $Width, $h)
    }
    if ($Pdf) { $p.SaveAs([IO.Path]::ChangeExtension($Pptx, ".pdf"), 32) }   # ppSaveAsPDF
    "rendered $($p.Slides.Count) slides"
    $p.Close()
} finally { $app.Quit() }
