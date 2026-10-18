//! `cargo xtask vectors`: regenerates `wit/vectors/vectors.txt`, the wire
//! format's conformance vectors, from the `bones-messages` codec.
//!
//! The vectors are written by `bones-messages`' own conformance test, which
//! otherwise checks the committed file against the codec on every run. This
//! command runs that test in write mode. Any line that changes in the diff is
//! a message every extension in existence now encodes or decodes differently,
//! so read the diff before committing it (ADR-029).

use crate::process::{cargo, repo_root, run_step};
use crate::Result;

pub fn run(args: &[String]) -> Result {
    if !args.is_empty() {
        return Err("usage: cargo xtask vectors".to_string());
    }
    let crate_dir = repo_root().join("crates/bones-messages");
    run_step(
        "Regenerating wit/vectors/vectors.txt",
        cargo(&crate_dir, &["test", "--test", "conformance"]).env("BONES_WRITE_VECTORS", "1"),
    )?;
    println!();
    println!(
        "Written wit/vectors/vectors.txt. Review the diff: every changed line is an ABI change."
    );
    Ok(())
}
