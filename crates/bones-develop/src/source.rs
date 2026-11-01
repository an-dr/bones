//! A bones checkout that a project's files are read from: a tag's source in
//! the cache, or a local checkout (ADR-047).
//!
//! - Why: a project is written for the engine it pins, so its templates,
//!   agent rules, and MCP configuration come from that engine's source
//!   rather than from whichever `bones-develop` is installed.
//! - The copies compiled into `bones-develop` are the fallback for a file
//!   the checkout does not have.

use std::path::{Path, PathBuf};

use crate::Result;

/// Where the tool's templates have lived, newest first.
const TEMPLATE_DIRS: [&str; 2] = [
    "crates/bones-develop/templates",
    "crates/bones-cli/templates",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    root: PathBuf,
}

impl Source {
    pub fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// `relative` from the checkout, or `fallback` when it has no such file.
    pub fn read(&self, relative: &str, fallback: &str) -> String {
        std::fs::read_to_string(self.root.join(relative)).unwrap_or_else(|_| fallback.to_string())
    }

    /// A template, by its path under `templates/`, or `fallback`.
    pub fn template(&self, relative: &str, fallback: &str) -> String {
        TEMPLATE_DIRS
            .iter()
            .find_map(|dir| std::fs::read_to_string(self.root.join(dir).join(relative)).ok())
            .unwrap_or_else(|| fallback.to_string())
    }

    /// The ABI line the checkout implements: `bones-messages`' version.
    pub fn abi(&self) -> Result<String> {
        let path = self.root.join("crates/bones-messages/Cargo.toml");
        let manifest = std::fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        manifest
            .lines()
            .find_map(|line| line.trim().strip_prefix("version = \""))
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_string)
            .ok_or_else(|| format!("{} has no version", path.display()))
    }
}

#[cfg(test)]
mod tests;
