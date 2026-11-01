//! `bones-develop self-update`: reinstall this tool with cargo (ADR-047).

use std::process::Command;

use crate::{repository, Result};

const USAGE: &str = "usage: bones-develop self-update [--tag <tag>]";

pub fn run(args: &[String]) -> Result {
    let tag = match args {
        [] => None,
        [flag, tag] if flag == "--tag" => Some(tag.as_str()),
        _ => return Err(USAGE.into()),
    };
    let mut install = Command::new("cargo");
    install.args(install_args(&repository::url(), tag));
    // Windows cannot overwrite a running executable, but it can rename one:
    // move this one aside so cargo can write the new one in its place.
    let aside = set_aside()?;
    let status = install
        .status()
        .map_err(|error| format!("running cargo install: {error}"));
    let succeeded = matches!(&status, Ok(status) if status.success());
    if let Some((current, old)) = aside {
        // Put this one back when cargo failed, or installed somewhere else
        // because this copy was not run from cargo's bin directory.
        if !succeeded || !current.exists() {
            let _ = std::fs::rename(&old, &current);
        }
    }
    match succeeded {
        true => Ok(()),
        false => Err(status
            .err()
            .unwrap_or_else(|| "cargo install failed; its output is above".into())),
    }
}

fn install_args<'a>(url: &'a str, tag: Option<&'a str>) -> Vec<&'a str> {
    let mut args = vec!["install", "--git", url, "--locked", "--force"];
    if let Some(tag) = tag {
        args.extend(["--tag", tag]);
    }
    args.push("bones-develop");
    args
}

#[cfg(windows)]
fn set_aside() -> Result<Option<(std::path::PathBuf, std::path::PathBuf)>> {
    let current =
        std::env::current_exe().map_err(|error| format!("finding bones-develop: {error}"))?;
    let old = current.with_extension("old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&current, &old)
        .map_err(|error| format!("moving {} aside: {error}", current.display()))?;
    Ok(Some((current, old)))
}

#[cfg(not(windows))]
fn set_aside() -> Result<Option<(std::path::PathBuf, std::path::PathBuf)>> {
    Ok(None)
}

#[cfg(test)]
mod tests;
