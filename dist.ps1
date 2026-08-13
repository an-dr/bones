#!/usr/bin/env pwsh
# Builds the app and every shipped extension, assembling a runnable dist/
# directory and a versioned, checksummed release archive beside it.
# Run with: pwsh dist.ps1
#
# examples/ is deliberately not built here: a distribution is the engine plus
# its reference extension, not a demo bundle. Build an example through its own
# examples/<name>/build.ps1, which assembles a self-contained dist/ beside it.
#
# -NoArchive stops after dist/, for the inner-loop case of running the engine
# you just built.
param([switch]$NoArchive)

$ErrorActionPreference = "Stop"

# crates/bones-engine/bones-kernel's platform module builds SDL3 from source via cmake; this sets up
# whatever compiler and generator this machine has. Shared with test.ps1.
. "$PSScriptRoot/scripts/native-build-env.ps1"
Initialize-NativeBuildEnvironment

$exeName = if ($IsWindows) { "bones.exe" } else { "bones" }
$dist = "dist"

# The engine version line (ADR-029), read from the workspace rather than
# repeated here, so a release cannot be labelled with a number nothing else
# agrees with.
$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages |
    Where-Object { $_.name -eq "bones" } |
    Select-Object -ExpandProperty version
if (-not $version) { throw "could not read the bones package version" }

$abiVersion = (Select-String -Path "wit/core.wit" -Pattern '^package\s+bones:core@(\S+);').Matches[0].Groups[1].Value.TrimEnd(';')

# Platform tag for the archive name. A bundle carries a native executable and a
# statically linked SDL, so it is only valid for the OS and architecture it was
# built on -- naming it accordingly is what stops the wrong one being shipped.
$os = if ($IsWindows) { "windows" } elseif ($IsMacOS) { "macos" } else { "linux" }
$arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64" { "x64" }
    "Arm64" { "arm64" }
    default { "$([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)".ToLowerInvariant() }
}
$bundleName = "bones-$version-$os-$arch"

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
#
# The whole directory, not just core.wit: WIT describes the calls but not the
# bytes they carry, so an author outside Rust also needs wire-format.md to build
# a payload and vectors/ to check that they built it correctly.
Write-Host "==> Copying the extension ABI..."
Copy-Item "wit" "$dist/wit" -Recurse

# The MIT licence requires its notice to travel with every copy of the
# software, so a bundle without it is not merely untidy -- it is not licensed.
Write-Host "==> Copying licence and notices..."
Copy-Item "LICENSE" "$dist/LICENSE"

# Third-party notices, generated rather than maintained by hand: the engine
# links its whole dependency tree statically, so the list changes whenever a
# dependency does, and a hand-written file would be wrong by the next release.
# cargo metadata is the source because it needs no extra tool installed.
$dependencies = (cargo metadata --format-version 1 --filter-platform (rustc -vV | Select-String '^host: ' | ForEach-Object { $_.ToString().Substring(6) }) | ConvertFrom-Json).packages |
    Where-Object { $_.source } |
    Sort-Object name, version

$notices = New-Object System.Text.StringBuilder
[void]$notices.AppendLine("Third-party notices for bones $version ($os-$arch)")
[void]$notices.AppendLine("")
[void]$notices.AppendLine("The bones executable statically links the packages below. Each remains under")
[void]$notices.AppendLine("its own licence; this file lists them so a redistributor can honour those")
[void]$notices.AppendLine("terms. bones itself is MIT -- see LICENSE.")
[void]$notices.AppendLine("")
[void]$notices.AppendLine("Generated by dist.ps1 from cargo metadata, for the host triple this bundle was")
[void]$notices.AppendLine("built for. Licences are identified by SPDX expression rather than reproduced;")
[void]$notices.AppendLine("where a licence requires its full text to travel with a binary, obtain that")
[void]$notices.AppendLine("text from the package's own repository, linked below.")
[void]$notices.AppendLine("")
foreach ($package in $dependencies) {
    $license = if ($package.license) { $package.license } else { "see $($package.license_file)" }
    [void]$notices.AppendLine("$($package.name) $($package.version)")
    [void]$notices.AppendLine("  licence:    $license")
    if ($package.repository) { [void]$notices.AppendLine("  repository: $($package.repository)") }
    [void]$notices.AppendLine("")
}
Set-Content -Path "$dist/THIRD-PARTY-NOTICES.txt" -Value $notices.ToString() -NoNewline

# A sample configuration, with every field at the value the engine would use
# anyway, so the file documents the defaults instead of changing behaviour by
# existing. Written here rather than committed so it cannot drift from
# crates/bones/src/config.rs the way a checked-in copy would.
Write-Host "==> Writing the sample configuration..."
@"
# bones configuration. Read from beside the executable, or from the path in
# BONES_CONFIG. Every value below is the default, so deleting this file changes
# nothing; delete the lines you do not want to pin.

extensions_dir = "extensions"
saves_dir = "states"

window_title = "bones"
window_width = 800
window_height = 600

# Presentation modules. renderer draws gfx/* batches; ui draws egui widget
# specs above it; web adds wry panels and needs a build with --features web.
renderer = true
ui = true
web = false

# Off by default: not every machine has a working audio device, and a
# non-game project has no use for the game-core simulation.
audio = false
game_core = false

# Extensions can save state unless this is true.
persistence_read_only = false

# Per-frame allowances per extension. Exceeding either quarantines it.
extension_max_inbound = 1024
extension_max_publishes = 1024
"@ | Set-Content -Path "$dist/bones.toml"

Write-Host "==> Writing the release README..."
@"
# bones $version

The bones engine and its reference extension, for $os-$arch.

Engine version $version; extension ABI bones:core@$abiVersion. The two move
independently -- see docs/adr/ADR-029 in the repository.

## Run it

    ./$exeName

It reads bones.toml from this directory and loads every .wasm file in
extensions/. Both paths resolve against the executable's own location, not the
shell's working directory, so a shortcut or a double-click behaves the same as
a terminal.

## Contents

    $exeName                  the engine
    bones.toml                configuration; every value is the default
    extensions/               extensions loaded at startup
    wit/core.wit              the extension ABI: the calls
    wit/wire-format.md        the extension ABI: the bytes those calls carry
    wit/vectors/              conformance vectors for the wire format
    LICENSE                   MIT, covering bones itself
    THIRD-PARTY-NOTICES.txt   the statically linked dependencies

## Write an extension

Drop any component built for wasm32-wasip2 into extensions/. In Rust, depend on
bones-wasm-sdk; in any other language, generate bindings from wit/core.wit and
build payloads per wit/wire-format.md.

An extension built against a different bones:core version will not load. That is
deliberate and is checked at instantiation, not at runtime.

## Where this came from

https://github.com/an-dr/bones
"@ | Set-Content -Path "$dist/README.md"

if ($NoArchive) {
    Write-Host ""
    Write-Host "Distribution ready: $dist/$exeName (extensions in $dist/extensions/)"
    Write-Host "Extension ABI: $dist/wit/core.wit (bones:core@$abiVersion)"
    return
}

# The archive is what a release publishes; dist/ is the working copy it is made
# from. Naming the directory inside the archive after the bundle means unpacking
# never scatters files into whatever directory the user happened to be in.
Write-Host "==> Archiving $bundleName..."
$staging = Join-Path ([System.IO.Path]::GetTempPath()) $bundleName
Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
Copy-Item $dist $staging -Recurse

$archive = "$bundleName.zip"
Remove-Item -Force $archive -ErrorAction SilentlyContinue
Compress-Archive -Path $staging -DestinationPath $archive
Remove-Item -Recurse -Force $staging

# Checksums, so a download can be verified against a value published separately
# from the file itself.
#
# This verifies a *download*, not a *build*: zip entries carry modification
# times, so two runs of this script produce different archive hashes from
# identical inputs. Publish the hash of the archive you actually uploaded.
# Byte-reproducible builds are a separate piece of work (docs/roadmap.md).
$hash = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -Path "$archive.sha256" -Value "$hash  $archive"

Write-Host ""
Write-Host "Distribution ready: $dist/$exeName (extensions in $dist/extensions/)"
Write-Host "Extension ABI: $dist/wit/core.wit (bones:core@$abiVersion)"
Write-Host "Release archive: $archive"
Write-Host "SHA256: $hash"
