//! Build automation for this repository, run as `cargo xtask <command>`
//! (the alias lives in `.cargo/config.toml`). Written in Rust so the
//! toolchain that builds the engine is the only thing it needs: no shell,
//! no script runtime, one implementation for every platform.
//!
//! Every command checks its prerequisites first (prereq.rs) and stops with
//! install hints rather than installing anything.

mod archive;
mod commits;
mod dist;
mod example;
mod files;
mod metadata;
mod native;
mod prereq;
mod process;
mod test;

use std::process::ExitCode;

/// Every command reports failure as a message the developer can act on;
/// main prints it and exits non-zero.
type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  check-commits   fail if a commit in a git log range credits an AI tool as an author
  doctor          check every prerequisite and say how to install what is missing
  dist            build the engine into dist/ and a release archive
                  (--no-archive: dist/ only; --features web: with web panels)
  example         build one example into its own dist/ (no name: list them)
  linux-packages  print the Debian/Ubuntu packages a Linux build needs
  test            build fixtures, run every gate and the whole test matrix";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check-commits") => commits::run(&args[1..]),
        Some("doctor") => prereq::run_doctor(),
        Some("dist") => dist::run(&args[1..]),
        Some("example") => example::run(&args[1..]),
        Some("linux-packages") => {
            println!("{}", prereq::LINUX_PACKAGES.join(" "));
            Ok(())
        }
        Some("test") => test::run(),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!();
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
