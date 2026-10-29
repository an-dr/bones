//! Finding the `bones` engine to run, and the ABI it implements.

use std::path::{Path, PathBuf};

use crate::Result;

/// The variable naming the engine when it is not beside `bones-cli`.
pub const ENGINE_VAR: &str = "BONES_ENGINE";

/// The engine: `explicit` when given, else `BONES_ENGINE`, else the `bones`
/// executable beside this one, as a release archive lays them out.
pub fn locate(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return existing(PathBuf::from(path), "--engine");
    }
    if let Ok(path) = std::env::var(ENGINE_VAR) {
        return existing(PathBuf::from(path), ENGINE_VAR);
    }
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(executable("bones"))));
    match beside {
        Some(path) if path.is_file() => Ok(path),
        _ => Err(format!(
            "no engine found beside bones-cli; unpack a release archive and run its bones-cli, \
             or set {ENGINE_VAR} to the path of a bones executable"
        )),
    }
}

/// The ABI line of the engine at `engine`, from the conformance vectors a
/// release archive ships beside it.
pub fn abi(engine: &Path) -> Option<String> {
    let vectors = engine.parent()?.join("wit/vectors/vectors.txt");
    abi_from_vectors(&std::fs::read_to_string(vectors).ok()?)
}

/// The version in the vectors' `# ABI line <version>,` header.
pub fn abi_from_vectors(vectors: &str) -> Option<String> {
    vectors.lines().find_map(|line| {
        let rest = line.strip_prefix("# ABI line ")?;
        Some(rest.split([',', ' ']).next()?.to_string())
    })
}

/// `stem` with the platform's executable suffix.
pub fn executable(stem: &str) -> String {
    format!("{stem}{}", std::env::consts::EXE_SUFFIX)
}

fn existing(path: PathBuf, source: &str) -> Result<PathBuf> {
    match path.is_file() {
        true => Ok(path),
        false => Err(format!(
            "{source} names {}, which does not exist",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests;
