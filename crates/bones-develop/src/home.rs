//! Where bones-develop keeps engines and their sources (ADR-047).

use std::path::PathBuf;

use crate::Result;

pub const HOME_VAR: &str = "BONES_HOME";

/// `BONES_HOME` when set, else `~/.bones`.
pub fn home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(HOME_VAR) {
        return Ok(PathBuf::from(path));
    }
    let user = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| {
            format!("no home directory found; set {HOME_VAR} to a folder for engines")
        })?;
    Ok(PathBuf::from(user).join(".bones"))
}
