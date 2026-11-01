//! `bones-develop upgrade <tag>`: move a project to another bones tag, in
//! whichever way it installs bones (ADR-047).
//!
//! - Local machine: the `v*` and `abi-v*` pins in its manifests are rewritten,
//!   the ABI read from the new tag's source.
//! - Submodule: the submodule is checked out at the tag.
//!
//! Either way its `AGENTS.md` and `.mcp.json` are rewritten from the new
//! tag, so the agent guidance always matches the engine it describes.

use std::path::Path;

use crate::new_project::{agents_md, project_mcp, PLUGIN_MCP, PLUGIN_MCP_PATH, RULES, RULES_PATH};
use crate::project::Project;
use crate::source::Source;
use crate::{engines, home, repository, submodule, Result};

#[cfg(test)]
mod tests;

const USAGE: &str = "usage: bones-develop upgrade <tag>";

/// The manifests that can carry a bones pin.
const MANIFESTS: [&str; 3] = [
    "extension/Cargo.toml",
    "checks/Cargo.toml",
    "app/Cargo.toml",
];

pub fn run(args: &[String]) -> Result {
    let [tag] = args else {
        return Err(USAGE.into());
    };
    let project = Project::find()?;
    if let Some(checkout) = project.engine_checkout() {
        // A submodule has a .git file; any other checkout is the user's own.
        if !checkout.join(".git").is_file() {
            return Err(format!(
                "the project builds against the checkout at {}, not a submodule; check out {tag} there yourself",
                checkout.display()
            ));
        }
        submodule::checkout_tag(&checkout, tag)?;
        let changed = update(&project.root, &Source::at(&checkout), &cli())?;
        println!(
            "Moved the submodule {} to {tag}{}. Commit the submodule change.",
            checkout.display(),
            agent_note(&changed)
        );
        return Ok(());
    }
    if project.pinned_engine().is_none() {
        return Err("the project pins no bones tag and has no bones checkout to move".into());
    }
    let source = Source::at(&engines::source(&home::home()?, &repository::url(), tag)?);
    let abi = source.abi()?;
    for relative in MANIFESTS {
        let path = project.root.join(relative);
        let Ok(manifest) = std::fs::read_to_string(&path) else {
            continue;
        };
        let updated = repin(&manifest, tag, &abi);
        if updated != manifest {
            write(&path, &updated)?;
        }
    }
    let changed = update(&project.root, &source, &cli())?;
    println!(
        "Pinned {tag} (ABI {abi}){}. The next dev builds its engine.",
        agent_note(&changed)
    );
    Ok(())
}

/// This `bones-develop`, for the `.mcp.json` it writes.
fn cli() -> String {
    std::env::current_exe()
        .map(|exe| exe.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| "bones-develop".to_string())
}

/// ", and updated AGENTS.md and .mcp.json from it", or nothing.
fn agent_note(changed: &[&str]) -> String {
    match changed.is_empty() {
        true => String::new(),
        false => format!(", and updated {} from it", changed.join(" and ")),
    }
}

/// `manifest` with every bones crate's git `tag = "..."` pin moved: `abi-v*` to `abi`,
/// any other to `tag`. Lines without a git pin are kept as they are.
fn repin(manifest: &str, tag: &str, abi: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in manifest.lines() {
        // Only bones' own crates: another git dependency keeps its tag.
        let bones = line.trim_start().starts_with("bones-") && line.contains("git = ");
        let pinned = bones.then(|| line.find("tag = \"")).flatten();
        let Some(at) = pinned else {
            lines.push(line.to_string());
            continue;
        };
        let start = at + "tag = \"".len();
        let Some(length) = line[start..].find('"') else {
            lines.push(line.to_string());
            continue;
        };
        let old = &line[start..start + length];
        let new = match old.starts_with("abi-v") {
            true => format!("abi-v{abi}"),
            false => tag.to_string(),
        };
        lines.push(format!(
            "{}{new}{}",
            &line[..start],
            &line[start + length..]
        ));
    }
    let mut text = lines.join("\n");
    if manifest.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn write(path: &Path, text: &str) -> Result {
    std::fs::write(path, text).map_err(|error| format!("writing {}: {error}", path.display()))
}

/// Rewrites `AGENTS.md` and `.mcp.json` from `source`, returning the names
/// of the files whose contents changed.
fn update(root: &Path, source: &Source, cli: &str) -> Result<Vec<&'static str>> {
    let files = [
        ("AGENTS.md", agents_md(&source.read(RULES_PATH, RULES))),
        (
            ".mcp.json",
            project_mcp(&source.read(PLUGIN_MCP_PATH, PLUGIN_MCP), cli),
        ),
    ];
    let mut changed = Vec::new();
    for (name, contents) in files {
        let path = root.join(name);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(contents.as_str()) {
            continue;
        }
        std::fs::write(&path, contents)
            .map_err(|error| format!("writing {}: {error}", path.display()))?;
        changed.push(name);
    }
    Ok(changed)
}
