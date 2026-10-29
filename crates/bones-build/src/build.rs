use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{wasm_artifact, INSTALL_TARGET, TARGET};

/// One release build of an extension crate for [`TARGET`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    dir: PathBuf,
    package: Option<String>,
    show_diagnostics: bool,
}

impl Build {
    /// A build of the crate, or workspace, in `dir`.
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            package: None,
            show_diagnostics: false,
        }
    }

    /// Builds only `name`, a package of the workspace in `dir`.
    pub fn package(mut self, name: impl Into<String>) -> Self {
        self.package = Some(name.into());
        self
    }

    /// Streams cargo's diagnostics to the terminal as they come, rather
    /// than returning them in the error.
    pub fn show_diagnostics(mut self) -> Self {
        self.show_diagnostics = true;
        self
    }

    /// Runs the build and returns the path of the `.wasm` it produced.
    ///
    /// Fails with cargo's diagnostics, and with [`INSTALL_TARGET`] when the
    /// target is what is missing.
    pub fn run(&self) -> Result<PathBuf, String> {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut command = Command::new(cargo);
        command.current_dir(&self.dir).args([
            "build",
            "--release",
            "--target",
            TARGET,
            "--message-format=json-render-diagnostics",
        ]);
        if let Some(package) = &self.package {
            command.args(["-p", package]);
        }
        if self.show_diagnostics {
            command.stderr(Stdio::inherit());
        }
        let output = command
            .output()
            .map_err(|error| format!("running cargo in {}: {error}", self.dir.display()))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            let hint = match stderr.contains(TARGET) && stderr.contains("may not be installed") {
                true => format!("\ninstall the target once: {INSTALL_TARGET}"),
                false => String::new(),
            };
            let shown = match self.show_diagnostics {
                true => "cargo's errors are above".to_string(),
                false => stderr.into_owned(),
            };
            return Err(format!(
                "building {} failed: {shown}{hint}",
                self.dir.display()
            ));
        }
        wasm_artifact(&String::from_utf8_lossy(&output.stdout)).ok_or_else(|| {
            format!(
                "building {} produced no .wasm; is the crate a cdylib?",
                self.dir.display()
            )
        })
    }
}
