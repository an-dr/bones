//! Builds a bones extension for `wasm32-wasip2` and finds the `.wasm` it
//! produced (ADR-043). `cargo xtask`, `bones-cli`, and `bones-test` all
//! build extensions; this is the one place that knows how.

#![warn(missing_docs)]

mod build;

use std::path::PathBuf;
use std::process::Command;

pub use build::Build;

/// The target every extension is built for.
pub const TARGET: &str = "wasm32-wasip2";

/// The command that installs [`TARGET`].
pub const INSTALL_TARGET: &str = "rustup target add wasm32-wasip2";

/// Whether rustup reports `target` as installed; `false` when rustup itself
/// is missing.
pub fn target_installed(target: &str) -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == target)
        })
}

/// The last `.wasm` among the `compiler-artifact` messages that
/// `cargo build --message-format=json` prints.
pub fn wasm_artifact(messages: &str) -> Option<PathBuf> {
    messages
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .flat_map(|message| message["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|name| name.as_str().map(PathBuf::from))
        .rfind(|path| path.extension().is_some_and(|ext| ext == "wasm"))
}

#[cfg(test)]
mod tests;
