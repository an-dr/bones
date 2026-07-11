#!/usr/bin/env pwsh
# Builds the app and every extension, assembling a runnable dist/ directory.
# Run with: pwsh dist.ps1
$ErrorActionPreference = "Stop"

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

Initialize-NativeBuildEnvironment

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
