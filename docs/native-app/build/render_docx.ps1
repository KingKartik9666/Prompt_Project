# Open a .docx in Word, refresh the table of contents and fields, save it, export a PDF,
# then rasterise the PDF pages to PNG with the Windows.Data.Pdf engine (for visual QA).
# Usage: render_docx.ps1 -Docx <path> [-PngDir <dir>] [-Width 1100]
param([Parameter(Mandatory)][string]$Docx, [string]$PngDir = "", [int]$Width = 1100)
$ErrorActionPreference = "Stop"
$Docx = (Resolve-Path $Docx).Path
$pdf = [IO.Path]::ChangeExtension($Docx, ".pdf")

$word = New-Object -ComObject Word.Application
$word.Visible = $false; $word.DisplayAlerts = 0
try {
    $d = $word.Documents.Open($Docx, $false, $false)
    foreach ($toc in $d.TablesOfContents) { $toc.Update() }
    $d.Fields.Update() | Out-Null
    $d.Repaginate()
    foreach ($toc in $d.TablesOfContents) { $toc.UpdatePageNumbers() }
    $d.Save()
    $d.ExportAsFixedFormat($pdf, 17)   # wdExportFormatPDF
    $pages = $d.ComputeStatistics(2)   # wdStatisticPages
    $d.Close($false)
} finally { $word.Quit() }
"pdf: $pdf ($pages pages)"

if ($PngDir) {
    New-Item -ItemType Directory -Force $PngDir | Out-Null
    Get-ChildItem $PngDir -Filter "page-*.png" | Remove-Item
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
    $null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
    $null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
    $ext = [System.WindowsRuntimeSystemExtensions]
    $asTaskOp = $ext.GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
    $asTaskAct = $ext.GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' } | Select-Object -First 1
    function Await-Op($op, [type]$t) { $task = $asTaskOp.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait(); $task.Result }
    function Await-Act($act) { $task = $asTaskAct.Invoke($null, @($act)); $task.Wait() }
    $file = Await-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync($pdf)) ([Windows.Storage.StorageFile])
    $doc = Await-Op ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
    for ($i = 0; $i -lt $doc.PageCount; $i++) {
        $page = $doc.GetPage($i)
        $opt = New-Object Windows.Data.Pdf.PdfPageRenderOptions
        $opt.DestinationWidth = $Width
        $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
        Await-Act ($page.RenderToStreamAsync($stream, $opt))
        $net = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($stream.GetInputStreamAt(0))
        $out = Join-Path $PngDir ("page-{0:D2}.png" -f ($i + 1))
        $fs = [IO.File]::Create($out); $net.CopyTo($fs); $fs.Close(); $net.Dispose(); $stream.Dispose(); $page.Dispose()
    }
    "png: $($doc.PageCount) pages -> $PngDir"
}
