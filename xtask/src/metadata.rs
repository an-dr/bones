//! Facts read from cargo and the repository rather than repeated in xtask,
//! so a release cannot disagree with the manifests it was built from.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::process::{cargo, read_output};
use crate::Result;

/// `cargo metadata --format-version 1` with `extra` arguments, parsed.
pub fn cargo_metadata(root: &Path, extra: &[&str]) -> Result<serde_json::Value> {
    let mut args = vec!["metadata", "--format-version", "1"];
    args.extend_from_slice(extra);
    let output = cargo(root, &args)
        .output()
        .map_err(|err| format!("could not run cargo metadata: {err}"))?;
    if !output.status.success() {
        return Err("cargo metadata failed".to_string());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("cargo metadata printed invalid JSON: {err}"))
}

/// The packages `metadata` lists.
pub fn packages(metadata: &serde_json::Value) -> impl Iterator<Item = &serde_json::Value> {
    metadata["packages"].as_array().into_iter().flatten()
}

/// The version of workspace package `name` -- for `bones`, the engine
/// version line (ADR-029).
pub fn package_version(root: &Path, name: &str) -> Result<String> {
    let metadata = cargo_metadata(root, &["--no-deps"])?;
    let version = packages(&metadata)
        .find(|package| package["name"] == name)
        .and_then(|package| package["version"].as_str())
        .map(str::to_string);
    version.ok_or_else(|| format!("could not read the {name} package version"))
}

/// The `bones:extension` ABI version declared in wit/extension.wit.
pub fn abi_version(root: &Path) -> Result<String> {
    let wit = root.join("wit/extension.wit");
    let text = fs::read_to_string(&wit)
        .map_err(|err| format!("could not read {}: {err}", wit.display()))?;
    text.lines()
        .find_map(|line| line.trim().strip_prefix("package bones:extension@"))
        .map(|rest| rest.trim_end_matches(';').trim().to_string())
        .ok_or_else(|| format!("{} declares no bones:extension package", wit.display()))
}

/// The target triple rustc builds for by default on this machine.
pub fn host_triple() -> Result<String> {
    read_output(Command::new("rustc").arg("-vV"))
        .and_then(|info| {
            info.lines()
                .find_map(|line| line.strip_prefix("host: "))
                .map(str::to_string)
        })
        .ok_or_else(|| "could not read the host triple from rustc -vV".to_string())
}
