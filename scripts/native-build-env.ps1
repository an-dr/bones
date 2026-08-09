#!/usr/bin/env pwsh
# Prepares the environment for building core/platform, which compiles SDL3
# from source through cmake. Dot-source it, then call the function:
#
#     . "$PSScriptRoot/scripts/native-build-env.ps1"
#     Initialize-NativeBuildEnvironment
#
# Shared by dist.ps1 and test.ps1. The per-extension build.ps1 scripts keep
# their own copy on purpose: an extension directory is meant to be copied out
# of this repository and still build.

# Any working C compiler does (MSVC, clang, clang-cl, gcc) -- prefer whatever
# this machine already has set up over assuming MSVC specifically.
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
