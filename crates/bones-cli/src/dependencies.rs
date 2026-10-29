use std::path::Path;

use crate::versions;

/// The dependency lines a new project's manifests carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependencies {
    /// For the extension: `bones-wasm-sdk`.
    pub sdk: String,
    /// For the checks: `bones-test`.
    pub test: String,
    /// For an `embed` project's app: `bones-kernel`.
    pub kernel: String,
}

const REPOSITORY: &str = "https://github.com/an-dr/bones";

impl Dependencies {
    /// The git tags of the release this `bones-cli` came from.
    pub fn tagged() -> Self {
        let engine = format!("tag = \"v{}\"", versions::ENGINE);
        let abi = format!("tag = \"abi-v{}\"", versions::abi());
        Self {
            sdk: format!("bones-wasm-sdk = {{ git = \"{REPOSITORY}\", {abi} }}"),
            test: format!("bones-test = {{ git = \"{REPOSITORY}\", {engine} }}"),
            kernel: format!(
                "bones-kernel = {{ git = \"{REPOSITORY}\", {engine}, features = [\"wasm\", \"devtools\"] }}"
            ),
        }
    }

    /// A local bones checkout, for a project built against unreleased work.
    pub fn local(checkout: &Path) -> Self {
        let path = |crate_dir: &str| {
            checkout
                .join(crate_dir)
                .to_string_lossy()
                .replace('\\', "/")
        };
        Self {
            sdk: format!(
                "bones-wasm-sdk = {{ path = \"{}\" }}",
                path("crates/bones-wasm-sdk")
            ),
            test: format!(
                "bones-test = {{ path = \"{}\" }}",
                path("crates/bones-test")
            ),
            kernel: format!(
                "bones-kernel = {{ path = \"{}\", features = [\"wasm\", \"devtools\"] }}",
                path("crates/bones-kernel")
            ),
        }
    }
}

#[cfg(test)]
mod tests;
