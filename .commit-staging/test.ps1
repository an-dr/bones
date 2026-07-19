#!/usr/bin/env pwsh
# Builds every extension fixture the test suite needs, then runs the whole
# workspace's tests. One command from a clean clone to green tests.
# Run with: pwsh test.ps1
$ErrorActionPreference = "Stop"

Write-Host "==> Building extension fixtures..."
pwsh ./extensions/build.ps1
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Running workspace tests..."
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Running bones-vocab's tests (excluded from the main workspace)..."
Push-Location shared/bones-vocab
try {
    cargo test
    if ($LASTEXITCODE -ne 0) { throw "bones-vocab tests failed" }
} finally {
    Pop-Location
}
