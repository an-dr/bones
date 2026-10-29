//! `cargo xtask wire-format`: renders `wit/wire-format.json`, the wire
//! format specification, as `wit/wire-format.md`, and writes the JSON Schema
//! it follows as `wit/wire-format.schema.json` (ADR-041).
//!
//! Before writing anything, it checks the specification against itself and
//! against the conformance vectors, which come from the `bones-messages`
//! codec, so a field the codec renamed or reordered fails here.
//!
//! The workspace tests generate both files in memory and fail when the
//! committed ones differ.

mod check;
pub mod model;
mod render;
pub mod vector;

use std::path::Path;

use crate::files::write_file;
use crate::process::repo_root;
use crate::Result;

pub use model::Spec;

/// The specification, relative to the repository root.
pub const SPEC: &str = "wit/wire-format.json";
/// The Markdown rendered from it.
pub const MARKDOWN: &str = "wit/wire-format.md";
/// The JSON Schema derived from the types that read it.
pub const SCHEMA: &str = "wit/wire-format.schema.json";

pub fn run(args: &[String]) -> Result {
    if !args.is_empty() {
        return Err("usage: cargo xtask wire-format".to_string());
    }
    let root = repo_root();
    let generated = generate(&root)?;
    write_file(&root.join(MARKDOWN), &generated.markdown)?;
    write_file(&root.join(SCHEMA), &generated.schema)?;
    println!("Written {MARKDOWN} and {SCHEMA}.");
    Ok(())
}

/// Both generated files, as they should be committed.
pub struct Generated {
    pub markdown: String,
    pub schema: String,
}

/// Reads and checks the specification under `root`, then renders it.
pub fn generate(root: &Path) -> Result<Generated> {
    let spec = load(root)?;
    let vectors = vector::parse(&read(&root.join("wit/vectors/vectors.txt"))?);
    check::check(&spec, &vectors)?;
    Ok(Generated {
        markdown: render::render(&spec),
        schema: schema(),
    })
}

/// The specification under `root`, parsed but not checked.
pub fn load(root: &Path) -> Result<Spec> {
    serde_json::from_str(&read(&root.join(SPEC))?)
        .map_err(|error| format!("parsing {SPEC}: {error}"))
}

/// The JSON Schema of `wire-format.json`, pretty-printed.
fn schema() -> String {
    let schema = schemars::schema_for!(Spec);
    let mut text = serde_json::to_string_pretty(&schema).expect("a schema serialises");
    text.push('\n');
    text
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|error| format!("reading {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
