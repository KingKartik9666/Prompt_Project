# Export the built HTML reports to PDF with headless Edge (Mermaid diagrams render from the CDN).
param([string]$Out = (Resolve-Path "$PSScriptRoot\..").Path)
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
$jobs = @(
  @{ src = "M2_report.html"; dst = "M2_PE-B65-13_AIBasedInteractiveQuantumAlgorithmLearningPlatform.pdf" },
  @{ src = "TechnicalReport.html"; dst = "Chica_TechnicalReport.pdf" }
)
foreach ($j in $jobs) {
  $url = "file:///" + (Join-Path $PSScriptRoot $j.src).Replace([char]92, [char]47)
  $pdf = Join-Path $Out $j.dst
  Start-Process -FilePath $edge -Wait -WindowStyle Hidden -ArgumentList "--headless=new","--disable-gpu","--no-pdf-header-footer","--run-all-compositor-stages-before-draw","--virtual-time-budget=25000","--print-to-pdf=$pdf",$url
  Get-Item $pdf | Select-Object Name, Length
}
