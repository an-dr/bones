#!/usr/bin/env pwsh
# Builds the single-instance demo. Run with: pwsh build.ps1
#
# The engine links SDL3, compiled from source, so the native build
# environment is prepared first, exactly as the repository's own test.ps1 does.
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path "$PSScriptRoot/../../..").Path
. "$repoRoot/scripts/native-build-env.ps1"
Initialize-NativeBuildEnvironment

Write-Host "==> Building the single-instance demo..."
cargo build --manifest-path "$PSScriptRoot/app/Cargo.toml" --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$exe = if ($IsWindows) { "single-instance-app.exe" } else { "single-instance-app" }
Write-Host ""
Write-Host "Start it, then start it again from another shell:"
Write-Host "  $PSScriptRoot/app/target/release/$exe"
Write-Host "  $PSScriptRoot/app/target/release/$exe --x y"
