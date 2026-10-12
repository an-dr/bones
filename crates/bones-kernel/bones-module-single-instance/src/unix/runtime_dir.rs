//! Where the socket and its lock live.

use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::PathBuf;

/// The socket path and the lock path for `app_uid`.
pub(super) fn paths(app_uid: &str) -> Result<(PathBuf, PathBuf), String> {
    let dir = directory()?;
    Ok((
        dir.join(format!("bones-single-instance-{app_uid}.sock")),
        dir.join(format!("bones-single-instance-{app_uid}.lock")),
    ))
}

/// `$XDG_RUNTIME_DIR` when it names a folder, which the session already made
/// private to the user; otherwise `<tmp>/bones-single-instance-<uid>`, created
/// private and refused if anyone else could have made or entered it.
fn directory() -> Result<PathBuf, String> {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        if runtime.is_dir() {
            return Ok(runtime);
        }
    }
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    let dir = std::env::temp_dir().join(format!("bones-single-instance-{uid}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("creating {}: {error}", dir.display())),
    }
    let metadata = std::fs::symlink_metadata(&dir)
        .map_err(|error| format!("reading {}: {error}", dir.display()))?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err(format!(
            "{} is not a folder private to this user",
            dir.display()
        ));
    }
    Ok(dir)
}
