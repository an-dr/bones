//! Filesystem steps the packaging commands share, each failing with the path
//! it was working on.

use std::fs;
use std::path::Path;

use crate::Result;

/// Deletes `dir` if it exists and creates it empty, parents included.
pub fn recreate_dir(dir: &Path) -> Result {
    if dir.exists() {
        fs::remove_dir_all(dir)
            .map_err(|err| format!("could not remove {}: {err}", dir.display()))?;
    }
    create_dir(dir)
}

pub fn create_dir(dir: &Path) -> Result {
    fs::create_dir_all(dir).map_err(|err| format!("could not create {}: {err}", dir.display()))
}

pub fn copy_file(from: &Path, to: &Path) -> Result {
    fs::copy(from, to).map(|_| ()).map_err(|err| {
        format!(
            "could not copy {} to {}: {err}",
            from.display(),
            to.display()
        )
    })
}

/// Copies `from` into `to` recursively, creating `to`.
pub fn copy_dir(from: &Path, to: &Path) -> Result {
    create_dir(to)?;
    let entries =
        fs::read_dir(from).map_err(|err| format!("could not read {}: {err}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("could not read {}: {err}", from.display()))?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn write_file(path: &Path, contents: &str) -> Result {
    fs::write(path, contents).map_err(|err| format!("could not write {}: {err}", path.display()))
}

/// The platform's executable name for `stem`.
pub fn exe_name(stem: &str) -> String {
    format!("{stem}{}", std::env::consts::EXE_SUFFIX)
}
