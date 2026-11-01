use std::path::Path;

use crate::repository;

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

impl Dependencies {
    /// The engine tag `tag` and the ABI tag of the line it implements.
    pub fn tagged(tag: &str, abi: &str) -> Self {
        let engine = format!("tag = \"{tag}\"");
        let abi = format!("tag = \"abi-v{abi}\"");
        let repository = repository::url();
        Self {
            sdk: format!("bones-wasm-sdk = {{ git = \"{repository}\", {abi} }}"),
            test: format!("bones-test = {{ git = \"{repository}\", {engine} }}"),
            kernel: format!(
                "bones-kernel = {{ git = \"{repository}\", {engine}, features = [\"wasm\", \"devtools\"] }}"
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
