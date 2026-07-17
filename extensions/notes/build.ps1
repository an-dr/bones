#!/usr/bin/env pwsh
# Builds the notes extension into a WASM component. Run with: pwsh build.ps1
$ErrorActionPreference = "Stop"

rustup target add wasm32-wasip2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo build --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Built: target/wasm32-wasip2/release/notes.wasm"
