use crate::install::remove_version_dir;
use crate::{current_version_dir, previous_version_dir};
use std::path::Path;

/// Drops the newest version folder so the previous one becomes current.
///
/// This is the escape hatch for a version that cannot start, and the whole
/// reason removing health supervision was safe: nothing guesses at a broken
/// build any more, so recovering from one is a thing the user asks for.
///
/// Refuses when there is nothing to fall back to. Deleting the only
/// installed version would leave the entry point with nothing to start,
/// which is worse than the broken version it replaced.
pub(crate) fn rollback(install_dir: &Path) -> Result<String, String> {
    let Some(current) = current_version_dir(install_dir) else {
        return Err(format!(
            "no version is installed under {}",
            install_dir.display()
        ));
    };
    let Some(previous) = previous_version_dir(install_dir) else {
        return Err(format!(
            "{} is the only installed version; there is nothing to roll back to",
            name_of(&current)
        ));
    };
    if std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|dir| dir.canonicalize().ok())
        == current.canonicalize().ok()
    {
        return Err("launch the previous version before removing the running version".into());
    }
    remove_version_dir(&current)?;
    Ok(format!(
        "removed {}, now running {}\n",
        name_of(&current),
        name_of(&previous)
    ))
}

fn name_of(dir: &Path) -> String {
    dir.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests;
