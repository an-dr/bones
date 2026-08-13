#!/usr/bin/env pwsh
# Builds the custom engine binary, the extension that speaks its vocabulary,
# and the stock `hello` extension, then assembles a runnable dist/ beside this
# script. Run with: pwsh build.ps1
#
# Unlike the extension examples, the executable here is *not* the shipped
# `bones`: it is this directory's own binary, which is the entire point.
$ErrorActionPreference = "Stop"

# The engine links SDL3, which sdl3-sys compiles from source via cmake. Any
# working C compiler does (MSVC, clang, clang-cl, gcc) -- prefer whatever this
# machine already has set up over assuming MSVC specifically.
function Initialize-NativeBuildEnvironment {
    # Ninja works uniformly with any of the compilers below and sidesteps
    # cmake's Visual-Studio-named-generator detection, which may not
    # recognize a newer VS version. Safe regardless of which compiler wins.
    if ((Get-Command ninja -ErrorAction SilentlyContinue) -and -not $env:CMAKE_GENERATOR) {
        $env:CMAKE_GENERATOR = "Ninja"
    }

    $compilers = "cl", "clang-cl", "clang", "gcc", "cc"
    if ($compilers | Where-Object { Get-Command $_ -ErrorAction SilentlyContinue }) {
        return # something is already on PATH -- respect it, don't override
    }
    if (-not $IsWindows) { return } # the MSVC fallback below is Windows-only

    $vcvarsall = @(
        "${env:ProgramFiles}\Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvarsall.bat",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvarsall.bat"
    ) | ForEach-Object { Get-Item -Path $_ -ErrorAction SilentlyContinue } | Select-Object -First 1
    if (-not $vcvarsall) {
        Write-Host "Note: no C compiler found and no vcvarsall.bat located; continuing as-is."
        return
    }

    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    Write-Host "==> No compiler on PATH; loading MSVC ($($vcvarsall.FullName) $arch)..."
    cmd /c "`"$($vcvarsall.FullName)`" $arch >nul 2>&1 && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            Set-Item -Path "env:$($matches[1])" -Value $matches[2]
        }
    }
}

$repoRoot = (Resolve-Path "$PSScriptRoot/../../..").Path
$exeName = if ($IsWindows) { "custom-engine.exe" } else { "custom-engine" }
$dist = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "dist"))
$exampleRoot = [IO.Path]::GetFullPath($PSScriptRoot)
if (-not $dist.StartsWith($exampleRoot + [IO.Path]::DirectorySeparatorChar)) {
    throw "Refusing to replace dist outside the custom-engine directory"
}

# Before any cargo invocation, not just before the engine one: cargo may build
# sdl3-sys as soon as it has a reason to, and a cmake configure that runs
# without a compiler in the environment fails outright rather than retrying.
Initialize-NativeBuildEnvironment

Write-Host "==> Ensuring the wasm32-wasip2 target is installed..."
rustup target add wasm32-wasip2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Building the host-probe extension..."
cargo build --manifest-path "$PSScriptRoot/extension/Cargo.toml" --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Packaged alongside so the window shows a stock extension and a custom one
# running in the same process, against the same engine.
Write-Host "==> Building the stock hello extension..."
cargo build --manifest-path "$repoRoot/crates/bones-extension-hello/Cargo.toml" --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Building the custom engine..."
cargo build --manifest-path "$PSScriptRoot/engine/Cargo.toml" --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (Test-Path -LiteralPath $dist) {
    Remove-Item -LiteralPath $dist -Recurse -Force
}
New-Item -ItemType Directory -Path "$dist/extensions" -Force | Out-Null
Copy-Item "$PSScriptRoot/engine/target/release/$exeName" "$dist/$exeName"
Copy-Item "$PSScriptRoot/bones.toml" "$dist/bones.toml"
Copy-Item "$PSScriptRoot/extension/target/wasm32-wasip2/release/host_probe.wasm" "$dist/extensions/host_probe.wasm"
Copy-Item "$repoRoot/crates/bones-extension-hello/target/wasm32-wasip2/release/bones_extension_hello.wasm" "$dist/extensions/bones_extension_hello.wasm"

Write-Host ""
Write-Host "Packaged: $dist/$exeName"
Write-Host "Run it, and the log shows host-probe asking the native module for facts it cannot reach itself."
