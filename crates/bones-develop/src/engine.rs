//! Finding the `bones` engine to run, and the ABI it implements.

use std::path::{Path, PathBuf};

use crate::project::Project;
use crate::{engines, home, Result};

/// The variable naming the engine when it is not beside `bones-develop`.
pub const ENGINE_VAR: &str = "BONES_ENGINE";

/// Whether `locate` may build a project's engine that is not cached yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    Allowed,
    CachedOnly,
}

/// The engine: `explicit` when given, else `BONES_ENGINE`, else the engine
/// for the tag `project` pins (ADR-047), else the `bones` executable beside
/// this one, as `cargo xtask dist` lays them out.
pub fn locate(explicit: Option<&str>, project: Option<&Project>, build: Build) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return existing(PathBuf::from(path), "--engine");
    }
    if let Ok(path) = std::env::var(ENGINE_VAR) {
        return existing(PathBuf::from(path), ENGINE_VAR);
    }
    if let Some(project) = project {
        if let Some(checkout) = project.engine_checkout() {
            let dir = project.root.join(".bones/engine");
            return engines::from_checkout(
                &checkout,
                &dir,
                project.uses_web(),
                build == Build::Allowed,
            );
        }
    }
    if let Some(tag) = project.and_then(Project::pinned_engine) {
        let web = project.is_some_and(Project::uses_web);
        let home = home::home()?;
        return match build {
            Build::Allowed => engines::install(&home, &tag, web),
            Build::CachedOnly => engines::cached(&home, &tag, web).ok_or_else(|| {
                let flag = if web { " --web" } else { "" };
                format!("the project pins {tag}, which is not built yet; run bones-develop engine install {tag}{flag}")
            }),
        };
    }
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(executable("bones"))));
    match beside {
        Some(path) if path.is_file() => Ok(path),
        _ => Err(format!(
            "no engine to run: not inside a project that pins a v* tag, and none beside bones-develop; \
             run bones-develop engine install <tag> and set {ENGINE_VAR} to the path it prints"
        )),
    }
}

/// The ABI line of the engine at `engine`, from the conformance vectors a
/// `dist/` build holds beside it.
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
