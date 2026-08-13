#!/usr/bin/env pwsh
# Builds every extension fixture the test suite needs, then runs the gates and
# the whole test matrix. One command from a clean clone to a release-green
# tree. Run with: pwsh test.ps1
#
# This script is where formatting, lints, and the optional-feature suite are
# enforced. There is no CI (docs/roadmap.md tracks adding it), so a gate that
# lives only in a reviewer's habits is a gate that does not exist.
$ErrorActionPreference = "Stop"

. "$PSScriptRoot/scripts/native-build-env.ps1"
Initialize-NativeBuildEnvironment

# Excluded from the root workspace (see its Cargo.toml) because they must also
# compile for wasm32-wasip2 guests, so every check below runs them separately,
# from inside their own directories.
$standaloneCrates = @(
    "crates/bones-messages"
    "crates/bones-wasm-sdk"
    # An example, but a wire contract too: it is the only place a custom
    # message vocabulary is defined, and it compiles without SDL, so checking
    # it costs seconds. The rest of examples/ is left to its own build.ps1,
    # since each would rebuild SDL from source.
    "examples/embedding/custom-engine/messages"
)

# Integration tests load these as prebuilt .wasm files by path, so they must
# exist before cargo test runs -- a missing one fails the test rather than
# skipping it. Paths are the crate directories; each builds to its own
# target/wasm32-wasip2/release/<name>.wasm.
$fixtures = @(
    "crates/bones-extension-hello"
    "examples/extensions/keyecho-demo"
    "examples/extensions/sprite-demo"
    "examples/extensions/runaway-demo"
    "examples/extensions/flood-demo"
    "examples/extensions/audio-demo"
    "examples/extensions/persistence-demo"
    # dashboard and metrics back the web-panel tests, which are gated behind
    # --features web on Windows. Built unconditionally: cheap, and it keeps
    # this script's output identical whether or not those tests run.
    "examples/extensions/dashboard-demo"
    "examples/extensions/metrics-demo"
)

function Invoke-Step {
    param([string]$Description, [scriptblock]$Step)

    Write-Host "==> $Description..."
    & $Step
    if ($LASTEXITCODE -ne 0) { throw "$Description failed" }
}

function Invoke-InEach {
    param([string[]]$Directories, [string]$Description, [scriptblock]$Step)

    foreach ($directory in $Directories) {
        Push-Location (Join-Path $PSScriptRoot $directory)
        try {
            Invoke-Step "$Description ($directory)" $Step
        } finally {
            Pop-Location
        }
    }
}

Invoke-Step "Ensuring the wasm32-wasip2 target is installed" { rustup target add wasm32-wasip2 }

# Each guest build below prints one `function signature mismatch: shutdown`
# warning from rust-lld. It is expected, understood, and cannot be fixed
# without an ABI break -- wit/README.md's known-issue section has the cause.
Write-Host "    (guest builds warn about the 'shutdown' export name; see wit/README.md)"

foreach ($fixture in $fixtures) {
    Push-Location (Join-Path $PSScriptRoot $fixture)
    try {
        # Built directly rather than through the fixture's own build.ps1:
        # that script also builds the engine and assembles a dist/ beside
        # itself, which the test suite does not need and which would rebuild
        # SDL once per fixture.
        Invoke-Step "Building fixture '$fixture'" { cargo build --target wasm32-wasip2 --release }
    } finally {
        Pop-Location
    }
}

# Deliberately not `cargo fmt --all`: that also formats local path
# dependencies, which here means rewriting the vendored pubsub-bus submodule
# even though the workspace excludes it. Asking cargo which packages the
# workspace actually owns keeps this correct when a crate is added.
$members = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages.name
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }

foreach ($member in $members) {
    Invoke-Step "Checking formatting of $member" { cargo fmt -p $member -- --check }
}
Invoke-InEach $standaloneCrates "Checking formatting" { cargo fmt -- --check }

Invoke-Step "Linting the workspace" {
    cargo clippy --workspace --all-targets --all-features -- -D warnings
}
Invoke-InEach $standaloneCrates "Linting" { cargo clippy --all-targets --all-features -- -D warnings }

# Both feature sets, because they exercise different code: the default build is
# what a distribution ships, and --all-features is the only thing that compiles
# and runs the optional wry web panels.
Invoke-Step "Running workspace tests (default features)" { cargo test --workspace }
Invoke-Step "Running workspace tests (all features)" { cargo test --workspace --all-features }

# --all-features so the SDK optional game-ui module is covered too.
Invoke-InEach $standaloneCrates "Running tests" { cargo test --all-features }

Invoke-Step "Checking the documentation build" {
    $env:RUSTDOCFLAGS = "-D warnings"
    cargo doc --workspace --all-features --no-deps
}

Write-Host ""
Write-Host "All gates and tests passed."
