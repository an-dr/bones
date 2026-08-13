#!/usr/bin/env pwsh
# Builds the notes extension into a WASM component, builds the bones
# engine, and assembles a runnable dist/notes-demo/ next to it. Run with:
# pwsh build.ps1
$ErrorActionPreference = "Stop"

rustup target add wasm32-wasip2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo build --manifest-path "$PSScriptRoot/Cargo.toml" --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Built: target/wasm32-wasip2/release/notes_demo.wasm"

# core/platform builds SDL3 from source via cmake. Any working C compiler
# does (MSVC, clang, clang-cl, gcc) -- prefer whatever this machine already
# has set up over assuming MSVC specifically.
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

    # Nothing found: fall back to loading MSVC via vcvarsall, since that's
    # the most likely thing already installed (even if not on PATH) on a
    # Windows machine with no compiler set up otherwise.
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
$exeName = if ($IsWindows) { "bones.exe" } else { "bones" }
$dist = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "dist"))
$demoRoot = [IO.Path]::GetFullPath($PSScriptRoot)
if (-not $dist.StartsWith($demoRoot + [IO.Path]::DirectorySeparatorChar)) {
    throw "Refusing to replace dist outside the notes-demo directory"
}

Write-Host "==> Building bones..."
Push-Location $repoRoot
try {
    Initialize-NativeBuildEnvironment
    cargo build -p bones --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

if (Test-Path -LiteralPath $dist) {
    Remove-Item -LiteralPath $dist -Recurse -Force
}
New-Item -ItemType Directory -Path "$dist/extensions" -Force | Out-Null
Copy-Item "$repoRoot/target/release/$exeName" "$dist/$exeName"
Copy-Item "$PSScriptRoot/bones.toml" "$dist/bones.toml"
Copy-Item "$PSScriptRoot/target/wasm32-wasip2/release/notes_demo.wasm" "$dist/extensions/notes_demo.wasm"

Write-Host ""
Write-Host "Packaged: $dist/$exeName (extensions/notes_demo.wasm alongside it)"
