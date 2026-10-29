use std::path::{Path, PathBuf};

use crate::Error;

/// Builds the extension crate in `dir` for `wasm32-wasip2` in release mode
/// and returns the path of the `.wasm` it produced.
///
/// Fails with cargo's own output when the build fails, and with what to do
/// when the target is not installed (`bones-build`).
pub fn build(dir: impl AsRef<Path>) -> Result<PathBuf, Error> {
    bones_build::Build::new(dir).run()
}
