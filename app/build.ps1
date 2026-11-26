# Builds Quantum Explainer: the WebAssembly interface first, then the native exe that embeds it.
#   .\build.ps1            release build -> target\release\QuantumExplainer.exe
#   .\build.ps1 -Dev       debug build -> target\debug\QuantumExplainer.exe (WebView devtools enabled)
#   .\build.ps1 -Run       build and start the app
param([switch]$Dev, [switch]$Run)
# Cargo writes progress to stderr; in Windows PowerShell 5.1 "Stop" would turn that into errors,
# so failures are detected through $LASTEXITCODE instead.
$ErrorActionPreference = "Continue"
$root = $PSScriptRoot
$llvm = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter "MartinStorsjo.LLVM-MinGW.MSVCRT*" -ErrorAction SilentlyContinue |
    ForEach-Object { Get-ChildItem $_.FullName -Directory } | Select-Object -First 1
if ($llvm) { $env:Path = "$($llvm.FullName)\bin;$env:Path" }
$env:Path = "$env:USERPROFILE\.cargo\bin;$root\.tools\bin;$env:Path"
$tc = "+stable-x86_64-pc-windows-gnullvm"
$profile = if ($Dev) { "debug" } else { "release" }
# (an if-expression would unwrap a one-item array into a string, and splatting a string passes its characters)
$flag = [string[]]@()
if (-not $Dev) { $flag = [string[]]@("--release") }

Push-Location $root
try {
    # 1. Interface -> WebAssembly
    cargo $tc build -p ui --target wasm32-unknown-unknown @flag
    if ($LASTEXITCODE) { throw "ui build failed" }
    $dist = "$root\crates\ui\dist"
    if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
    New-Item -ItemType Directory -Force "$dist\fonts" | Out-Null
    wasm-bindgen --target web --no-typescript --out-dir $dist --out-name ui "$root\target\wasm32-unknown-unknown\$profile\ui.wasm"
    if ($LASTEXITCODE) { throw "wasm-bindgen failed" }
    Copy-Item "$root\crates\ui\index.html", "$root\crates\ui\boot.js", "$root\crates\ui\styles\app.css" $dist
    Copy-Item "$root\crates\ui\assets\fonts\*.woff2" "$dist\fonts"

    # 2. Native app (embeds dist/)
    cargo $tc build -p desktop @flag
    if ($LASTEXITCODE) { throw "desktop build failed" }
    $exe = "$root\target\$profile\QuantumExplainer.exe"
    Write-Host "Built $exe ($([math]::Round((Get-Item $exe).Length / 1MB, 1)) MB)"
    if ($Run) { Start-Process $exe }
} finally { Pop-Location }
