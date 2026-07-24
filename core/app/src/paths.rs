//! Resolves configured paths (`bones.toml` itself, `extensions_dir`,
//! `saves_dir`). `bones.toml`'s own location decides the base every other
//! relative path resolves against: next to the running executable by
//! default, so a shipped `dist/` behaves the same regardless of launch
//! directory. Overridable via the `BONES_CONFIG` env var for workflows
//! where the exe lives deep inside a build directory far from any config a
//! developer wants to edit directly (e.g. an embedder running `cargo run`
//! straight against a vendored `bones` checkout, never producing a `dist/`
//! at all) — `extensions_dir`/`saves_dir` then resolve against
//! `BONES_CONFIG`'s own directory instead of the exe's, so a repo-root
//! `bones.toml` can point at a repo-root `extensions/` without either
//! living next to the built binary.

use std::path::{Path, PathBuf};

const CONFIG_ENV_VAR: &str = "BONES_CONFIG";

/// Where `bones.toml` itself is read from.
pub fn config_path() -> PathBuf {
    resolve_config_path(std::env::var_os(CONFIG_ENV_VAR).map(PathBuf::from))
}

/// Resolves a config value (`extensions_dir`, `saves_dir`) against
/// `bones.toml`'s own base directory, wherever that config came from.
pub fn config_relative(path: impl AsRef<Path>) -> PathBuf {
    resolve_config_relative(
        path.as_ref(),
        std::env::var_os(CONFIG_ENV_VAR).map(PathBuf::from),
    )
}

fn resolve_config_path(env_override: Option<PathBuf>) -> PathBuf {
    env_override.unwrap_or_else(|| relative_to_exe("bones.toml"))
}

fn resolve_config_relative(path: &Path, env_override: Option<PathBuf>) -> PathBuf {
    let base = match env_override {
        Some(config_path) => config_path.parent().map(Path::to_path_buf),
        None => exe_dir(),
    };
    join_relative(path, base)
}

fn relative_to_exe(path: impl AsRef<Path>) -> PathBuf {
    join_relative(path.as_ref(), exe_dir())
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// Absolute paths pass through unchanged; a relative path joins onto
/// `base` when one is available, otherwise stays relative (today's
/// CWD-relative behavior, unchanged).
fn join_relative(path: &Path, base: Option<PathBuf>) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match base {
        Some(dir) => dir.join(path),
        None => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests;
