#!/usr/bin/env pwsh
# Builds the flooding extension and its healthy peer, then packages a
# directly runnable demonstration. Run with: pwsh build.ps1
$ErrorActionPreference = "Stop"

rustup target add wasm32-wasip2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo build --manifest-path "$PSScriptRoot/Cargo.toml" --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --manifest-path "$PSScriptRoot/../../crates/bones-extension-hello/Cargo.toml" --target wasm32-wasip2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Built: target/wasm32-wasip2/release/flood_demo.wasm"

function Initialize-NativeBuildEnvironment {
    if ((Get-Command ninja -ErrorAction SilentlyContinue) -and -not $env:CMAKE_GENERATOR) {
        $env:CMAKE_GENERATOR = "Ninja"
    }

    $compilers = "cl", "clang-cl", "clang", "gcc", "cc"
    if ($compilers | Where-Object { Get-Command $_ -ErrorAction SilentlyContinue }) {
        return
    }
    if (-not $IsWindows) { return }

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

$repoRoot = (Resolve-Path "$PSScriptRoot/../..").Path
$exeName = if ($IsWindows) { "bones.exe" } else { "bones" }
$dist = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "dist"))
$demoRoot = [IO.Path]::GetFullPath($PSScriptRoot)
if (-not $dist.StartsWith($demoRoot + [IO.Path]::DirectorySeparatorChar)) {
    throw "Refusing to replace dist outside the flood_demo directory"
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
Copy-Item "$PSScriptRoot/target/wasm32-wasip2/release/flood_demo.wasm" "$dist/extensions/flood_demo.wasm"
Copy-Item "$PSScriptRoot/../../crates/bones-extension-hello/target/wasm32-wasip2/release/bones_extension_hello.wasm" "$dist/extensions/bones_extension_hello.wasm"

Write-Host ""
Write-Host "Packaged: $dist/$exeName (flood_demo + healthy hello peer)"
