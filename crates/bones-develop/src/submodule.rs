//! A project that vendors bones as a git submodule (ADR-047).
//!
//! - The submodule is checked out at a tag, so a clone with `--recursive`
//!   holds the engine source, and the project's path dependencies point at it.
//! - `protocol.file.allow=always` lets the repository be a local path, as
//!   `BONES_REPOSITORY` is in tests and CI; git refuses it by default.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;

/// Makes `root` a git repository unless it is inside one already.
pub fn ensure_repository(root: &Path) -> Result {
    let inside = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .is_ok_and(|output| output.status.success());
    match inside {
        true => Ok(()),
        false => git(root, &["init", "--quiet"]),
    }
}

/// Adds the repository at `url` as a submodule at `path` (relative to
/// `root`), checked out at `tag` with its own submodules.
pub fn add(root: &Path, path: &str, url: &str, tag: &str) -> Result<PathBuf> {
    eprintln!("Adding {url} at {tag} as the submodule {path}");
    git(
        root,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            url,
            path,
        ],
    )?;
    let checkout = root.join(path);
    checkout_tag(&checkout, tag)?;
    Ok(checkout)
}

/// Moves the submodule at `checkout` to `tag`, fetching it when needed.
pub fn checkout_tag(checkout: &Path, tag: &str) -> Result {
    git(checkout, &["fetch", "--quiet", "--tags", "origin"])?;
    git(
        checkout,
        &[
            "-c",
            "advice.detachedHead=false",
            "checkout",
            "--quiet",
            tag,
        ],
    )?;
    git(
        checkout,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
            "--recursive",
            "--quiet",
        ],
    )
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .map_err(|error| format!("running git: {error}; install git"))?;
    match status.success() {
        true => Ok(()),
        false => Err(format!(
            "git {} failed in {}; its output is above",
            args.join(" "),
            dir.display()
        )),
    }
}
