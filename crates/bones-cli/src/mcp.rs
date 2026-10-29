//! `bones-cli mcp`: an MCP server on stdio over the engine's devtools
//! channel, so an AI agent can drive and observe the running engine.
//!
//! - `protocol` answers the JSON-RPC messages MCP uses.
//! - `tools` declares the tools and carries each call out.

mod protocol;
mod tools;

use std::io::{BufRead, Write};

use crate::Result;

pub use tools::Tools;

/// Serves one client until stdin closes: one JSON-RPC message per line in,
/// one answer per line out.
pub fn run() -> Result {
    let mut tools = Tools::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("reading stdin: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = protocol::answer(&line, &mut tools) {
            writeln!(stdout, "{answer}").map_err(|error| format!("writing stdout: {error}"))?;
            stdout.flush().ok();
        }
    }
    tools.stop_engine();
    Ok(())
}
