//! `bones-cli`: the developer tools that ship beside the engine (ADR-043).
//!
//! Each subcommand lives in its own module; this file only dispatches and
//! reports a failure as one message with what to do about it.

mod base64;
mod catalogue;
mod dependencies;
mod dev;
mod devtools_client;
mod doctor;
mod engine;
mod hex;
mod kind;
mod mcp;
mod new_project;
mod project;
mod screenshot;
mod versions;

use std::process::ExitCode;

/// Every command reports failure as a message the user can act on.
type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "\
usage: bones-cli <command>

commands:
  new <name> [--kind game|egui|web|embed] [--bones-path <checkout>]
                 write a project that builds and passes its checks
  doctor         check this machine and project, and say how to fix what is missing
  build          build the extension and copy it where the engine loads it
  dev [--engine <path>] [--devtools <port>]
                 build, run the engine, rebuild and hot-reload on every change
  screenshot [--out <file.png>] [--frames <n>] [--press <key>]... [--window] [--engine <path>]
                 build, run offscreen, press keys, step frames, save the frame as a PNG;
                 --window: a visible window, captured whole with its web panels
  mcp            serve the devtools channel to an AI agent over MCP on stdio

The engine is the bones executable beside bones-cli, or BONES_ENGINE.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    // `--help` anywhere is a request for the usage, never a name or a value.
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = match args.first().map(String::as_str) {
        Some("new") => new_project::run(rest),
        Some("doctor") => doctor::run(),
        Some("build") => project::Project::find().and_then(|project| project.build().map(|_| ())),
        Some("dev") => dev::run(rest),
        Some("screenshot") => screenshot::run(rest),
        Some("mcp") => mcp::run(),
        Some("--version") => {
            println!("bones-cli {} (ABI {})", versions::ENGINE, versions::abi());
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
