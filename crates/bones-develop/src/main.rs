//! `bones-develop`: the developer tools that ship beside the engine (ADR-043).
//!
//! Each subcommand lives in its own module; this file only dispatches and
//! reports a failure as one message with what to do about it.

mod add;
mod base64;
mod catalogue;
mod dependencies;
mod dev;
mod devtools_client;
mod doctor;
mod engine;
mod engines;
mod hex;
mod home;
mod kind;
mod mcp;
mod new_project;
mod project;
mod repository;
mod screenshot;
mod self_update;
mod source;
mod submodule;
mod upgrade;
mod versions;

use std::process::ExitCode;

/// Every command reports failure as a message the user can act on.
type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "\
usage: bones-develop <command>

commands:
  new <name> [--kind game|egui|web|embed] [--tag <tag>] [--submodule [<path>] | --bones-path <checkout>]
                 write a project that builds and passes its checks, pinned to the newest
                 v* tag unless told otherwise; --submodule vendors bones at that tag (./bones)
  add [--tag <tag>] [--submodule [<path>]]
                 make the program in this crate an embedder: bones-kernel pinned to a tag
  doctor         check this machine and project, and say how to fix what is missing
  engine install <tag> [--web] | list | remove <tag>
                 build an engine from a tag into ~/.bones (BONES_HOME), list or remove them
  build          build the extension and copy it where the engine loads it
  dev [--engine <path>] [--devtools <port>]
                 build, run the engine, rebuild and hot-reload on every change
  screenshot [--out <file.png>] [--frames <n>] [--press <key>]... [--window] [--engine <path>]
                 build, run offscreen, press keys, step frames, save the frame as a PNG;
                 --window: a visible window, captured whole with its web panels
  mcp            serve the devtools channel to an AI agent over MCP on stdio
  upgrade <tag>  move the project and its agent files to another bones tag
  self-update [--tag <tag>]
                 reinstall bones-develop with cargo install

The engine is --engine, else BONES_ENGINE, else the engine for the tag the project pins.";

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
        Some("add") => add::run(rest),
        Some("self-update") => self_update::run(rest),
        Some("upgrade") => upgrade::run(rest),
        Some("doctor") => doctor::run(),
        Some("engine") => engines::run(rest),
        Some("build") => project::Project::find().and_then(|project| project.build().map(|_| ())),
        Some("dev") => dev::run(rest),
        Some("screenshot") => screenshot::run(rest),
        Some("mcp") => mcp::run(),
        Some("--version") => {
            println!(
                "bones-develop {} (ABI {})",
                versions::ENGINE,
                versions::abi()
            );
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
