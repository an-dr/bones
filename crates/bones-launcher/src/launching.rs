//! Selects and starts an installed application; never modifies its installation.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::selection::{current_version_dir, previous_version_dir};

/// Starts the latest installed application and forwards its arguments unchanged.
///
/// `--previous` as the first argument selects the previous version for recovery
/// and is consumed here. No version is deleted or installed.
pub fn run(app_exe: &str, install_dir: &Path, args: &[String]) -> Result<(), String> {
    let (version_dir, args) = if args.first().is_some_and(|arg| arg == "--previous") {
        (previous_version_dir(install_dir), &args[1..])
    } else {
        (current_version_dir(install_dir), args)
    };
    let version_dir = version_dir.ok_or_else(|| {
        format!(
            "no selected version is installed under {}",
            install_dir.display()
        )
    })?;
    let exe = version_dir.join(app_exe);
    spawn_detached(&exe, args)
        .map_err(|error| format!("could not launch {}: {error}", exe.display()))
}

/// Spawns `exe` with its own fresh stdio rather than inheriting this
/// process's: a detached GUI child that outlives its parent would otherwise
/// keep the parent's stdout/stderr handles open for as long as it keeps
/// running, which is indefinite -- observed directly, where a caller
/// redirecting the launcher's stderr to a file hung waiting for that file's
/// handle count to reach zero long after the launcher itself had exited.
///
/// The child is left running rather than returned: dropping a `Child`
/// neither kills nor waits for it, and a Windows child outlives its parent
/// by default, so this process exiting immediately afterwards does not
/// affect the app it just started.
fn spawn_detached(exe: &Path, args: &[String]) -> std::io::Result<()> {
    let mut command = Command::new(exe);
    command.args(args);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().map(drop)
}
