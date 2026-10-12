use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) fn ordered_version_dirs(install_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(install_dir) else {
        return Vec::new();
    };
    bones_versioning::order_version_folders(entries.flatten().filter_map(|entry| {
        entry.file_type().ok()?.is_dir().then(|| {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (entry.path(), modified)
        })
    }))
}

pub(crate) fn current_version_dir(install_dir: &Path) -> Option<PathBuf> {
    ordered_version_dirs(install_dir).into_iter().next()
}

pub(crate) fn previous_version_dir(install_dir: &Path) -> Option<PathBuf> {
    ordered_version_dirs(install_dir).into_iter().nth(1)
}

#[cfg(test)]
mod tests;
