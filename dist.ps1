#!/usr/bin/env pwsh
# Builds the app and every shipped extension, assembling a runnable dist/
# directory. Run with: pwsh dist.ps1
#
# examples/ is deliberately not built here: a distribution is the engine plus
# its reference extension, not a demo bundle. Build an example through its own
# examples/<name>/build.ps1, which assembles a self-contained dist/ beside it.
$ErrorActionPreference = "Stop"

# crates/bones-engine/bones-kernel's platform module builds SDL3 from source via cmake; this sets up
# whatever compiler and generator this machine has. Shared with test.ps1.
. "$PSScriptRoot/scripts/native-build-env.ps1"
Initialize-NativeBuildEnvironment

$exeName = if ($IsWindows) { "bones.exe" } else { "bones" }
$dist = "dist"

Write-Host "==> Building app..."
cargo build -p bones --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Remove-Item -Recurse -Force $dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$dist/extensions" -Force | Out-Null
Copy-Item "target/release/$exeName" "$dist/$exeName"

# Only crates matching the bones-extension-* prefix reserved for shipped
# extensions (ADR-030, ADR-030), never examples/ -- see the note at the top
# of this file.
Get-ChildItem -Path "crates" -Directory -Filter "bones-extension-*" | ForEach-Object {
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

    # cargo names the artifact after the crate, with underscores for hyphens.
    $artifactName = $name -replace '-', '_'
    $wasm = Join-Path $_.FullName "target/wasm32-wasip2/release/$artifactName.wasm"
    Copy-Item $wasm "$dist/extensions/$artifactName.wasm"
}

# The ABI ships with the engine that implements it. An extension author in any
# language needs core.wit to build against, and pairing it with the binary is
# what makes the bones:core version in it verifiable rather than advisory --
# wasmtime refuses to instantiate a component whose imported interface version
# differs from this engine's.
Write-Host "==> Copying the extension ABI..."
New-Item -ItemType Directory -Path "$dist/wit" -Force | Out-Null
Copy-Item "wit/core.wit" "$dist/wit/core.wit"
Copy-Item "wit/README.md" "$dist/wit/README.md"

$abiVersion = (Select-String -Path "wit/core.wit" -Pattern '^package\s+bones:core@(\S+);').Matches[0].Groups[1].Value.TrimEnd(';')

Write-Host ""
Write-Host "Distribution ready: $dist/$exeName (extensions in $dist/extensions/)"
Write-Host "Extension ABI: $dist/wit/core.wit (bones:core@$abiVersion)"
