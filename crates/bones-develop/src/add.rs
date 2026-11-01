//! `bones-develop add`: make an existing Rust program an embedder, with the
//! engine as a library pinned to a tag (ADR-047).
//!
//! - `bones-kernel` comes from the tag's git repository, or from a path into
//!   a submodule that vendors bones at that tag (`--submodule`).
//! - A new extension product starts from `bones-develop new` instead, which
//!   writes its checks and dev loop as well.
//!
//! Dependencies go in through `cargo add`, which owns the manifest's format.

use std::path::Path;
use std::process::Command;

use crate::new_project::newest_tag;
use crate::{repository, submodule, Result};

#[cfg(test)]
mod tests;

const USAGE: &str = "usage: bones-develop add [--tag <tag>] [--submodule [<path>]]";

pub fn run(args: &[String]) -> Result {
    let (tag, vendored) = parse_options(args)?;
    if !Path::new("Cargo.toml").is_file() {
        return Err("no Cargo.toml here; run bones-develop add in your program's crate, or bones-develop new for a new project".into());
    }
    let tag = match tag {
        Some(tag) => tag,
        None => newest_tag()?,
    };
    let url = repository::url();
    let pin = match &vendored {
        Some(path) => {
            submodule::ensure_repository(Path::new("."))?;
            submodule::add(Path::new("."), path, &url, &tag)?;
            Pin::Path(path.clone())
        }
        None => Pin::Git(url),
    };
    let mut add = pin.args("bones-kernel", &tag);
    add.extend(["--features".into(), "wasm,devtools".into()]);
    cargo_add(&add)?;
    println!("Added bones-kernel {tag}. Build the engine in main with bones_kernel::Kernel; examples/embedding in the bones repository shows how.");
    Ok(())
}

/// Where `cargo add` takes a bones crate from.
enum Pin {
    Git(String),
    /// The submodule's path, relative to the crate.
    Path(String),
}

impl Pin {
    /// `cargo add` arguments for `name`; `tag` applies to a git pin only.
    fn args(&self, name: &str, tag: &str) -> Vec<String> {
        match self {
            Self::Git(url) => [name, "--git", url, "--tag", tag]
                .map(String::from)
                .to_vec(),
            Self::Path(path) => vec![
                name.to_string(),
                "--path".into(),
                format!("{path}/crates/{name}"),
            ],
        }
    }
}

/// `--tag <tag>` and `--submodule [<path>]`, in any order.
fn parse_options(options: &[String]) -> Result<(Option<String>, Option<String>)> {
    let (mut tag, mut vendored) = (None, None);
    let mut options = options.iter().peekable();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--tag" => tag = Some(options.next().ok_or_else(|| USAGE.to_string())?.clone()),
            "--submodule" => {
                let path = options.next_if(|value| !value.starts_with("--"));
                vendored = Some(path.cloned().unwrap_or_else(|| "bones".to_string()));
            }
            _ => return Err(USAGE.into()),
        }
    }
    Ok((tag, vendored))
}

fn cargo_add(args: &[String]) -> Result {
    let status = Command::new("cargo")
        .arg("add")
        .args(args)
        .status()
        .map_err(|error| format!("running cargo add: {error}"))?;
    match status.success() {
        true => Ok(()),
        false => Err("cargo add failed; its output is above".into()),
    }
}
