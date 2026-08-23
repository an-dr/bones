#!/usr/bin/env pwsh
# Builds the demo app twice -- 1.0.0 and 1.1.0 -- installs the first, and
# publishes the second as a manifest the installed copy can find. Run with:
# pwsh build.ps1
#
# Two versions is the point. A self-update example with one version can only
# show the check saying "up to date", which proves nothing about the part that
# is hard: replacing a running application with a newer copy of itself.
$ErrorActionPreference = "Stop"

$app = Join-Path $PSScriptRoot "app"
$dist = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "dist"))
$exampleRoot = [IO.Path]::GetFullPath($PSScriptRoot)
if (-not $dist.StartsWith($exampleRoot + [IO.Path]::DirectorySeparatorChar)) {
    throw "Refusing to replace dist outside the self-updating-app directory"
}

$appExe = if ($IsWindows) { "demo-app.exe" } else { "demo-app" }
$launcherExe = if ($IsWindows) { "demo-launcher.exe" } else { "demo-launcher" }
$entryExe = if ($IsWindows) { "demo.exe" } else { "demo" }

# Set as real environment variables rather than left to app/.cargo/config.toml
# alone. Cargo merges [env] from every ancestor directory, so a clone that sits
# inside another cargo project inherits that project's values for the same
# keys; a real environment variable outranks all of them. The config file is
# still there, and is what a normal standalone checkout uses.
function Build-Version([string]$version) {
    Write-Host "==> Building demo-app $version..."
    $env:DEMO_APP_VERSION = $version
    $env:UPGRADER_INSTALL_DIR_NAME = ".demo-app"
    $env:UPGRADER_INSTALL_ENV_VAR = "DEMO_APP_INSTALL_DIR"
    $env:UPGRADER_LAUNCHER_STEM = "demo"
    $env:UPGRADER_APP_STEM = "demo-app"
    $env:UPGRADER_LAUNCHER_VERSION = $version
    cargo build --manifest-path "$app/Cargo.toml" --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Remove-Item Env:\DEMO_APP_VERSION
}

if (Test-Path -LiteralPath $dist) {
    Remove-Item -LiteralPath $dist -Recurse -Force
}

# The newer build first, packaged as the asset the older one will find. It is
# a plain zip of a version folder's contents, which is what extract_version
# unpacks.
Build-Version "1.1.0"
$staging = Join-Path $dist "staging"
New-Item -ItemType Directory -Path $staging -Force | Out-Null
Copy-Item "$app/target/release/$appExe" "$staging/$appExe"
Copy-Item "$app/target/release/$launcherExe" "$staging/$launcherExe"

New-Item -ItemType Directory -Path "$dist/app" -Force | Out-Null
$asset = Join-Path $dist "app/demo-app-1.1.0.zip"
Compress-Archive -Path "$staging/*" -DestinationPath $asset -Force
Remove-Item -LiteralPath $staging -Recurse -Force

# The installed tree: the launcher at the root under the name users type, and
# the app inside a version folder. This is the layout the launcher walks.
Build-Version "1.0.0"
New-Item -ItemType Directory -Path "$dist/app/1.0.0" -Force | Out-Null
Copy-Item "$app/target/release/$appExe" "$dist/app/1.0.0/$appExe"
Copy-Item "$app/target/release/$launcherExe" "$dist/app/1.0.0/$launcherExe"
Copy-Item "$app/target/release/$launcherExe" "$dist/app/$entryExe"

# The manifest the app fetches. `sha256` is optional in the format and
# supplied here, because a download that does not match is refused outright.
$hash = (Get-FileHash -Path $asset -Algorithm SHA256).Hash.ToLower()
$assetUrl = "file://" + $asset.Replace("\", "/")
@{ version = "1.1.0"; url = $assetUrl; sha256 = $hash } |
    ConvertTo-Json | Set-Content -LiteralPath "$dist/app/manifest.json"

Write-Host ""
Write-Host "Installed 1.0.0 at: $dist/app"
Write-Host ""
Write-Host "Point the app at this install and run it through the launcher:"
Write-Host "  `$env:DEMO_APP_INSTALL_DIR = '$dist/app'"
Write-Host "  $dist/app/$entryExe"
Write-Host ""
Write-Host "The first run reports 1.0.0 and stages 1.1.0; the second starts 1.1.0."
Write-Host "  $dist/app/$entryExe --version    what is installed"
Write-Host "  $dist/app/$entryExe --rollback   go back to the previous version"
