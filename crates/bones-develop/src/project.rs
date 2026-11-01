use std::path::{Path, PathBuf};

use crate::Result;

/// A project `bones-develop new` laid out: `bones.toml` beside an `extension/`
/// crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    /// The extension crate's package name.
    pub name: String,
}

impl Project {
    /// The project containing the current directory.
    pub fn find() -> Result<Self> {
        let start = std::env::current_dir().map_err(|error| error.to_string())?;
        Self::find_from(&start)
    }

    /// The nearest directory at or above `start` that holds `bones.toml` and
    /// `extension/Cargo.toml`.
    pub fn find_from(start: &Path) -> Result<Self> {
        let root = start
            .ancestors()
            .find(|dir| {
                dir.join("bones.toml").is_file() && dir.join("extension/Cargo.toml").is_file()
            })
            .ok_or_else(|| {
                format!(
                    "{} is not inside a bones project (bones.toml beside extension/); \
                     create one with bones-develop new <name>",
                    start.display()
                )
            })?
            .to_path_buf();
        let manifest = read(&root.join("extension/Cargo.toml"))?;
        let name = package_name(&manifest)
            .ok_or("extension/Cargo.toml has no package name")?
            .to_string();
        Ok(Self { root, name })
    }

    /// Whether the project runs its own native binary (`embed`).
    pub fn has_app(&self) -> bool {
        self.root.join("app/Cargo.toml").is_file()
    }

    /// The ABI tag the extension pins, or `None` for a path dependency.
    pub fn pinned_abi(&self) -> Option<String> {
        let manifest = read(&self.root.join("extension/Cargo.toml")).ok()?;
        let start = manifest.find("tag = \"abi-v")? + "tag = \"abi-v".len();
        let end = manifest[start..].find('"')? + start;
        Some(manifest[start..end].to_string())
    }

    /// Where the engine loads the extension from: the project's
    /// `bones.toml` names `.bones/extensions`.
    /// The engine tag (`v...`) the project's manifests pin, if they pin one
    /// rather than a local checkout.
    pub fn pinned_engine(&self) -> Option<String> {
        ["checks/Cargo.toml", "app/Cargo.toml"]
            .iter()
            .find_map(|manifest| {
                let manifest = read(&self.root.join(manifest)).ok()?;
                let start = manifest.find("tag = \"v")? + "tag = \"".len();
                let end = manifest[start..].find('"')? + start;
                Some(manifest[start..end].to_string())
            })
    }

    /// The bones checkout the project's path dependencies point into, such as
    /// its submodule, when it depends on one rather than on tags.
    pub fn engine_checkout(&self) -> Option<PathBuf> {
        ["checks/Cargo.toml", "app/Cargo.toml"]
            .iter()
            .find_map(|relative| {
                let manifest_path = self.root.join(relative);
                let manifest = read(&manifest_path).ok()?;
                manifest.lines().find_map(|line| {
                    let start = line.find("path = \"")? + "path = \"".len();
                    let end = line[start..].find('"')? + start;
                    let crate_dir = &line[start..end];
                    let checkout = crate_dir
                        .strip_suffix("/crates/bones-test")
                        .or_else(|| crate_dir.strip_suffix("/crates/bones-kernel"))?;
                    let dir = manifest_path.parent()?.join(checkout).canonicalize().ok()?;
                    // Windows' canonical form has a verbatim prefix that reads badly.
                    let text = dir.to_string_lossy();
                    let dir =
                        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text).to_string());
                    dir.join("crates/bones/Cargo.toml").is_file().then_some(dir)
                })
            })
    }

    /// Whether the project's `bones.toml` turns web panels on.
    pub fn uses_web(&self) -> bool {
        read(&self.root.join("bones.toml")).is_ok_and(|config| {
            config
                .lines()
                .any(|line| line.split('#').next().unwrap_or("").replace(' ', "") == "web=true")
        })
    }

    pub fn installed_wasm(&self) -> PathBuf {
        self.root
            .join(".bones/extensions")
            .join(format!("{}.wasm", self.name.replace('-', "_")))
    }

    /// Builds the extension for `wasm32-wasip2` and installs it where the
    /// engine loads it; a running engine hot-reloads it.
    pub fn build(&self) -> Result<PathBuf> {
        let built = bones_build::Build::new(&self.root)
            .package(&self.name)
            .show_diagnostics()
            .run()?;
        let installed = self.installed_wasm();
        std::fs::create_dir_all(installed.parent().unwrap())
            .map_err(|error| format!("creating .bones/extensions: {error}"))?;
        std::fs::copy(&built, &installed).map_err(|error| {
            format!(
                "copying {} to {}: {error}",
                built.display(),
                installed.display()
            )
        })?;
        // Status goes to stderr: under `bones-develop mcp`, stdout is the protocol.
        eprintln!("Installed {}", installed.display());
        Ok(installed)
    }
}

/// The `name = "…"` of a manifest's `[package]`.
fn package_name(manifest: &str) -> Option<&str> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .find_map(|line| line.trim().strip_prefix("name = \""))
        .and_then(|rest| rest.strip_suffix('"'))
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|error| format!("reading {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
