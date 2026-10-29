//! `cargo xtask plugin-docs`: regenerates the `bones-user` agent plugin's
//! documentation reference, the product-facing documents in one file, so a
//! client that loads the plugin carries them whole.
//!
//! The workspace tests regenerate it in memory and fail when the committed
//! file differs, so it cannot fall behind the documents it is made of.

use std::path::Path;

use crate::files::write_file;
use crate::process::repo_root;
use crate::Result;

/// Where the reference lives, relative to the repository root.
pub const OUTPUT: &str = "agent-plugins/bones-user/skills/bones/references/docs.md";

/// The documents, in reading order: the tools, then the contract they rest
/// on. The rules and the skill are in the plugin already.
const SOURCES: [&str; 7] = [
    "crates/bones-cli/README.md",
    "crates/bones-test/README.md",
    "crates/bones-wasm-sdk/README.md",
    "docs/design/extensions.md",
    "crates/bones-kernel/bones-module-devtools/README.md",
    "wit/README.md",
    "wit/wire-format.md",
];

pub fn run(args: &[String]) -> Result {
    if !args.is_empty() {
        return Err("usage: cargo xtask plugin-docs".to_string());
    }
    let root = repo_root();
    write_file(&root.join(OUTPUT), &generate(&root)?)?;
    println!("Written {OUTPUT}.");
    Ok(())
}

/// Each source under a `# <path>` heading, with line endings normalised.
pub fn generate(root: &Path) -> Result<String> {
    let mut text = String::from(
        "# bones: the product-facing documentation in one file\n\n\
         Generated from the files named below by `cargo xtask plugin-docs`. Do not edit by hand. \
         Relative links in them point into the bones repository.\n",
    );
    for source in SOURCES {
        let path = root.join(source);
        let content = std::fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        text.push_str(&format!("\n\n# {source}\n\n"));
        text.push_str(content.replace("\r\n", "\n").trim_end());
        text.push('\n');
    }
    Ok(text)
}

#[cfg(test)]
mod tests;
