//! `bones-cli doctor`: checks the machine and the project, and says how to
//! fix each problem. Installs nothing.

use std::process::Command;

use crate::engine;
use crate::project::Project;
use crate::Result;

pub fn run() -> Result {
    let mut problems = 0;
    let mut report = |ok: bool, what: &str, fix: &str| match ok {
        true => println!("ok    {what}"),
        false => {
            problems += 1;
            println!("FAIL  {what}\n      {fix}");
        }
    };

    let cargo = output("cargo", &["--version"]);
    report(
        cargo.is_some(),
        "cargo",
        "install Rust from https://rustup.rs",
    );
    report(
        bones_build::target_installed(bones_build::TARGET),
        "the wasm32-wasip2 target",
        bones_build::INSTALL_TARGET,
    );

    let engine = engine::locate(None);
    report(
        engine.is_ok(),
        "an engine to run",
        engine.as_ref().err().map(String::as_str).unwrap_or(""),
    );
    let engine_abi = engine.as_ref().ok().and_then(|path| engine::abi(path));

    if let Ok(project) = Project::find() {
        println!(
            "      project {} at {}",
            project.name,
            project.root.display()
        );
        if project.pinned_abi().is_none() {
            println!("      ABI pin not checked: the extension uses a path dependency, not an abi-v* tag");
        } else if engine_abi.is_none() {
            println!("      ABI pin not checked: no wit/ beside the engine to read its ABI from");
        }
        if let (Some(pinned), Some(engine_abi)) = (project.pinned_abi(), &engine_abi) {
            report(
                pinned == *engine_abi,
                &format!("the extension pins ABI {pinned}; the engine implements {engine_abi}"),
                &format!("set tag = \"abi-v{engine_abi}\" in extension/Cargo.toml, or use the engine that matches"),
            );
        }
    }

    match problems {
        0 => Ok(()),
        n => Err(format!("{n} problem(s) above")),
    }
}

/// A command's stdout, or `None` when it cannot run or fails.
fn output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
