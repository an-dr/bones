//! `bones-develop engine`: build engines from a tag and cache them (ADR-047).
//!
//! - `sources/<tag>`: a shallow clone of the tag, with submodules.
//! - `engines/<tag>[-web]`: the `bones` executable, with the tag's `wit/`
//!   beside it so the ABI it implements can be read.
//! - `target/`: one cargo target directory for every build.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::executable;
use crate::home::home;
use crate::{repository, Result};

#[cfg(test)]
mod tests;

const USAGE: &str = "usage: bones-develop engine install <tag> [--web] | list | remove <tag>";

pub fn run(args: &[String]) -> Result {
    match args {
        [command, tag, rest @ ..] if command == "install" => {
            let web = match rest {
                [] => false,
                [flag] if flag == "--web" => true,
                _ => return Err(USAGE.into()),
            };
            let engine = install(&home()?, tag, web)?;
            println!("{}", engine.display());
            Ok(())
        }
        [command] if command == "list" => list(&home()?),
        [command, tag] if command == "remove" => remove(&home()?, tag),
        _ => Err(USAGE.into()),
    }
}

/// The engine for `tag` if it is already built.
pub fn cached(home: &Path, tag: &str, web: bool) -> Option<PathBuf> {
    let engine = engine_dir(home, tag, web).ok()?.join(executable("bones"));
    engine.is_file().then_some(engine)
}

/// The engine for `tag`, built on first use.
pub fn install(home: &Path, tag: &str, web: bool) -> Result<PathBuf> {
    let dir = engine_dir(home, tag, web)?;
    let engine = dir.join(executable("bones"));
    if engine.is_file() {
        return Ok(engine);
    }
    let source = source(home, &repository::url(), tag)?;
    eprintln!("Building the bones engine {tag} -- the first build of a tag takes a few minutes");
    build_into(&source, &home.join("target"), &dir, web)
}

/// The engine of a local checkout, such as a project's submodule, built into
/// `dir` with its own target directory beside it. cargo makes a rebuild of an
/// unchanged checkout quick, so `Build::Allowed` always runs it.
pub fn from_checkout(checkout: &Path, dir: &Path, web: bool, may_build: bool) -> Result<PathBuf> {
    let engine = dir.join(executable("bones"));
    if !may_build {
        return match engine.is_file() {
            true => Ok(engine),
            false => Err(format!(
                "the engine of {} is not built yet; bones-develop dev builds it",
                checkout.display()
            )),
        };
    }
    build_into(checkout, &dir.join("target"), dir, web)
}

/// Builds `bones` from `source` in `target`, then places it and the source's
/// `wit/` in `dir`.
fn build_into(source: &Path, target: &Path, dir: &Path, web: bool) -> Result<PathBuf> {
    let mut build = Command::new("cargo");
    build
        .current_dir(source)
        .env("CARGO_TARGET_DIR", target)
        .args(["build", "--release", "-p", "bones"]);
    if web {
        build.args(["--features", "web"]);
    }
    run_step(&mut build, "cargo build")?;
    std::fs::create_dir_all(dir).map_err(|error| format!("creating {}: {error}", dir.display()))?;
    // A rebuilt checkout may have changed its ABI: replace, never merge.
    let _ = std::fs::remove_dir_all(dir.join("wit"));
    copy_dir(&source.join("wit"), &dir.join("wit"))?;
    let built = target.join("release").join(executable("bones"));
    let engine = dir.join(executable("bones"));
    std::fs::copy(&built, &engine)
        .map_err(|error| format!("copying {}: {error}", built.display()))?;
    Ok(engine)
}

/// The source of `tag` from the repository at `url`, cloned on first use.
pub fn source(home: &Path, url: &str, tag: &str) -> Result<PathBuf> {
    check_tag(tag)?;
    let dir = home.join("sources").join(tag);
    if dir.join("Cargo.toml").is_file() {
        return Ok(dir);
    }
    let partial = home.join("sources").join(format!("{tag}.partial"));
    // A clone interrupted earlier leaves this behind; start it again.
    let _ = std::fs::remove_dir_all(&partial);
    std::fs::create_dir_all(home.join("sources"))
        .map_err(|error| format!("creating {}: {error}", home.display()))?;
    eprintln!("Fetching {tag} from {url}");
    run_step(
        Command::new("git")
            .args([
                "-c",
                "advice.detachedHead=false",
                "clone",
                "--quiet",
                "--depth",
                "1",
            ])
            .args([
                "--branch",
                tag,
                "--recurse-submodules",
                "--shallow-submodules",
                url,
            ])
            .arg(&partial),
        "git clone",
    )?;
    std::fs::rename(&partial, &dir)
        .map_err(|error| format!("moving the clone to {}: {error}", dir.display()))?;
    Ok(dir)
}

/// The repository's `v*` tags, oldest first by version.
pub fn remote_tags() -> Result<Vec<String>> {
    let url = repository::url();
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--refs", &url, "v*"])
        .output()
        .map_err(|error| format!("running git: {error}; install git"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-remote {url} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            line.split_once("refs/tags/")
                .map(|(_, tag)| tag.to_string())
        })
        .collect();
    tags.sort_by_key(|tag| version_key(tag));
    Ok(tags)
}

fn list(home: &Path) -> Result {
    let mut cached: Vec<String> = std::fs::read_dir(home.join("engines"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    cached.sort_by_key(|name| version_key(name));
    println!("Installed in {}:", home.join("engines").display());
    for name in &cached {
        println!("  {name}");
    }
    if cached.is_empty() {
        println!("  (none)");
    }
    println!("Available from {}:", repository::url());
    for tag in remote_tags()? {
        println!("  {tag}");
    }
    Ok(())
}

fn remove(home: &Path, tag: &str) -> Result {
    let dirs = [
        engine_dir(home, tag, false)?,
        engine_dir(home, tag, true)?,
        home.join("sources").join(tag),
    ];
    let mut removed = false;
    for dir in dirs.iter().filter(|dir| dir.exists()) {
        std::fs::remove_dir_all(dir)
            .map_err(|error| format!("removing {}: {error}", dir.display()))?;
        removed = true;
    }
    match removed {
        true => Ok(()),
        false => Err(format!("{tag} is not installed")),
    }
}

fn engine_dir(home: &Path, tag: &str, web: bool) -> Result<PathBuf> {
    check_tag(tag)?;
    let name = if web {
        format!("{tag}-web")
    } else {
        tag.to_string()
    };
    Ok(home.join("engines").join(name))
}

/// A tag names one folder in the cache: no separators, no parent references.
fn check_tag(tag: &str) -> Result {
    let ok = !tag.is_empty()
        && !tag.starts_with('-')
        && !tag.contains("..")
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    match ok {
        true => Ok(()),
        false => Err(format!("{tag:?} is not a tag name")),
    }
}

/// `v3.10.0` sorts after `v3.9.0`; a name that is not a version sorts first.
pub fn version_key(tag: &str) -> Vec<u64> {
    tag.trim_start_matches('v')
        .split(['.', '-'])
        .map_while(|part| part.parse().ok())
        .collect()
}

fn run_step(command: &mut Command, what: &str) -> Result {
    let status = command
        .status()
        .map_err(|error| format!("running {what}: {error}"))?;
    match status.success() {
        true => Ok(()),
        false => Err(format!("{what} failed; its output is above")),
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result {
    std::fs::create_dir_all(to).map_err(|error| format!("creating {}: {error}", to.display()))?;
    let entries =
        std::fs::read_dir(from).map_err(|error| format!("reading {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("reading {}: {error}", from.display()))?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .map_err(|error| format!("copying {}: {error}", entry.path().display()))?;
        }
    }
    Ok(())
}
