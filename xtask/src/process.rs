//! Running child processes and locating programs and the repository.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;

/// The repository root: xtask/ sits directly inside it.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent directory")
        .to_path_buf()
}

/// A `cargo` command running in `dir`. Uses the cargo that launched xtask,
/// so a toolchain override (`cargo +nightly xtask ...`) carries through.
pub fn cargo(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(dir).args(args);
    command
}

/// Announces `description`, runs `command` to completion with inherited
/// output, and fails with the description when it exits non-zero.
pub fn run_step(description: &str, command: &mut Command) -> Result {
    println!("==> {description}...");
    let status = command.status().map_err(|err| {
        format!(
            "{description}: could not start {}: {err}",
            command.get_program().to_string_lossy()
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{description} failed ({status})"))
    }
}

/// Runs `command` and returns its stdout, or `None` when it cannot start or
/// exits non-zero. For probes, where failure is an answer, not an error.
pub fn read_output(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Whether `program` resolves on `PATH`, honouring `PATHEXT` on Windows.
pub fn is_on_path(program: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    let extensions: Vec<String> = if cfg!(windows) {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
            .split(';')
            .map(str::to_string)
            .collect()
    } else {
        vec![String::new()]
    };
    env::split_paths(&path).any(|dir| {
        extensions
            .iter()
            .any(|extension| dir.join(format!("{program}{extension}")).is_file())
    })
}
