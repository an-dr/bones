#!/usr/bin/env pwsh
# Builds every extension fixture the test suite needs, then runs all tests.
# One command from a clean clone to green tests. Run with: pwsh test.ps1
$ErrorActionPreference = "Stop"

. "$PSScriptRoot/scripts/native-build-env.ps1"
Initialize-NativeBuildEnvironment

# Integration tests load these as prebuilt .wasm files by path, so they must
# exist before cargo test runs -- a missing one fails the test rather than
# skipping it. Paths are the crate directories; each builds to its own
# target/wasm32-wasip2/release/<name>.wasm.
$fixtures = @(
    "extensions/hello"
    "examples/keyecho"
    "examples/sprite_demo"
    "examples/runaway_demo"
    "examples/flood_demo"
    "examples/audio_demo"
    "examples/persistence_demo"
    # dashboard and metrics back the web-panel test, which is gated behind
    # --features web on Windows. Built unconditionally: cheap, and it keeps
    # this script's output identical whether or not that test runs.
    "examples/dashboard"
    "examples/metrics"
)

Write-Host "==> Ensuring the wasm32-wasip2 target is installed..."
rustup target add wasm32-wasip2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

foreach ($fixture in $fixtures) {
    Write-Host "==> Building fixture '$fixture'..."
    Push-Location (Join-Path $PSScriptRoot $fixture)
    try {
        # Built directly rather than through the fixture's own build.ps1:
        # that script also builds the engine and assembles a dist/ beside
        # itself, which the test suite does not need and which would rebuild
        # SDL once per fixture.
        cargo build --target wasm32-wasip2 --release
        if ($LASTEXITCODE -ne 0) { throw "fixture build failed: $fixture" }
    } finally {
        Pop-Location
    }
}

Write-Host "==> Running workspace tests..."
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Excluded from the root workspace (see its Cargo.toml) because they must
# also compile for wasm32-wasip2 guests, so their tests need their own run.
foreach ($crate in "shared/bones-messages", "shared/game-ui") {
    Write-Host "==> Running $crate tests (excluded from the main workspace)..."
    Push-Location (Join-Path $PSScriptRoot $crate)
    try {
        cargo test
        if ($LASTEXITCODE -ne 0) { throw "$crate tests failed" }
    } finally {
        Pop-Location
    }
}

Write-Host ""
Write-Host "All tests passed."
