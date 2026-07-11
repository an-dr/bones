#!/usr/bin/env pwsh
# Builds the app and every extension, assembling a runnable dist/ directory.
# Run with: pwsh dist.ps1
$ErrorActionPreference = "Stop"

$exeName = if ($IsWindows) { "bones.exe" } else { "bones" }
$dist = "dist"

Write-Host "==> Building app..."
cargo build -p app --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Remove-Item -Recurse -Force $dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$dist/extensions" -Force | Out-Null
Copy-Item "target/release/$exeName" "$dist/$exeName"

Get-ChildItem -Path "extensions" -Directory | ForEach-Object {
    $name = $_.Name
    $buildScript = Join-Path $_.FullName "build.ps1"
    if (-not (Test-Path $buildScript)) { return }

    Write-Host "==> Building extension '$name'..."
    Push-Location $_.FullName
    try {
        pwsh ./build.ps1
        if ($LASTEXITCODE -ne 0) { throw "build failed for extension '$name'" }
    } finally {
        Pop-Location
    }

    $wasm = Join-Path $_.FullName "target/wasm32-wasip2/release/$name.wasm"
    Copy-Item $wasm "$dist/extensions/$name.wasm"
}

Write-Host ""
Write-Host "Distribution ready: $dist/$exeName (extensions in $dist/extensions/)"
