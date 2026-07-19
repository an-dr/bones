#!/usr/bin/env pwsh
# Builds one extension into a WASM component, or every extension if no name
# is given. Run with: pwsh build.ps1 [name]
$ErrorActionPreference = "Stop"

function Build-Extension {
    param([string]$Dir)
    $name = Split-Path $Dir -Leaf
    Write-Host "==> Building extension '$name'..."
    Push-Location $Dir
    try {
        rustup target add wasm32-wasip2
        if ($LASTEXITCODE -ne 0) { throw "rustup target add failed for '$name'" }
        cargo build --target wasm32-wasip2 --release
        if ($LASTEXITCODE -ne 0) { throw "build failed for '$name'" }
        Write-Host "Built: $Dir/target/wasm32-wasip2/release/$name.wasm"
    } finally {
        Pop-Location
    }
}

if ($args.Count -gt 0) {
    Build-Extension (Join-Path $PSScriptRoot $args[0])
} else {
    Get-ChildItem -Path $PSScriptRoot -Directory | Where-Object {
        Test-Path (Join-Path $_.FullName "Cargo.toml")
    } | ForEach-Object {
        Build-Extension $_.FullName
    }
}
