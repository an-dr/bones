//! The application half: it knows its own version, and can replace itself.
//!
//! Everything here is what an application actually has to write. The version
//! arithmetic, the version folders and the entry-point replacement are
//! `bones-upgrader`'s; what an app supplies is its identity, a way to fetch,
//! and the decision about when to update.
//!
//! Run it through the launcher, not directly -- that is the whole point of
//! the two-binary shape, and `--help` explains why.

use std::path::Path;

use bones_upgrader::{
    current_version_dir, default_install_dir, download_asset_verified, extract_version,
    fetch_manifest, host_identity, is_newer, Fetch,
};

/// This build's version. A real app uses `env!("CARGO_PKG_VERSION")`; the
/// example hardcodes it so `build.ps1` can produce two versions from one
/// source tree and show an update actually happening.
const VERSION: &str = match option_env!("DEMO_APP_VERSION") {
    Some(version) => version,
    None => "1.0.0",
};

/// Reads a `file://` URL, so the example needs no network and no server.
///
/// A real host wires this to whatever it already has -- the engine's `os`
/// module offers `fetch_url` over HTTPS, which is one line here. The trait is
/// this narrow precisely so that either is easy.
struct FileFetch;

impl Fetch for FileFetch {
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        let path = url
            .strip_prefix("file://")
            .ok_or_else(|| format!("this demo only reads file:// URLs, got {url}"))?;
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(format!(
                "application/octet-stream;base64,{}",
                base64(&bytes)
            ))),
            // A missing manifest is a normal "nothing published yet", not a
            // failure -- which is why the trait returns Option.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// The example's own base64, so it carries no dependency beyond the upgrader.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn main() {
    let identity = host_identity();
    println!("demo-app {VERSION}");
    println!("  install dir name: {}", identity.install_dir_name);
    println!("  launcher: {}", identity.launcher_exe());

    let Some(install_dir) = default_install_dir(&identity) else {
        println!("  (no install dir resolved; running from a build tree)");
        return;
    };
    println!("  installed at:     {}", install_dir.display());
    if let Some(current) = current_version_dir(&install_dir) {
        println!("  current version:  {}", current.display());
    }

    // The manifest lives beside the install in this example. A real app points
    // at an HTTPS URL it controls.
    let manifest_url = format!("file://{}", install_dir.join("manifest.json").display());
    match update(&install_dir, &manifest_url) {
        Ok(Some(version)) => {
            println!("  staged {version}; the launcher will start it next run");
        }
        Ok(None) => println!("  up to date"),
        Err(error) => println!("  update check failed: {error}"),
    }
}

/// Checks the manifest, and stages the new version when there is one.
///
/// Staging rather than replacing is what makes this safe: the running app is
/// never overwritten, and the launcher picks the newest version folder on the
/// next start. A failure here costs the update, not the installation.
fn update(install_dir: &Path, manifest_url: &str) -> Result<Option<String>, String> {
    let manifest = fetch_manifest(&FileFetch, manifest_url)?;
    if !is_newer(VERSION, &manifest.version) {
        return Ok(None);
    }
    println!("  found {} (running {VERSION})", manifest.version);
    // Refuses a checksum mismatch outright: a truncated download is treated
    // exactly like a network failure, never silently unpacked.
    let asset = download_asset_verified(&FileFetch, &manifest)?;
    extract_version(&asset, install_dir, &manifest.version)?;
    Ok(Some(manifest.version))
}
