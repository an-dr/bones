//! Resolves configured paths (bones.toml, extensions_dir, saves_dir)
//! against the running executable's own directory rather than the
//! process's current working directory, so `bones.exe` behaves the same
//! whether launched by double-click, shortcut, or from an arbitrary shell.

use std::path::{Path, PathBuf};

pub fn relative_to_exe(path: impl AsRef<Path>) -> PathBuf {
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
