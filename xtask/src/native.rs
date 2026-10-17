//! Prepares the environment for building SDL3 from source through cmake,
//! which `bones-kernel-infra` does. The settings go into xtask's own
//! environment, so every cargo it starts inherits them.

use std::env;
use std::path::PathBuf;
use std::process::Command;

use crate::process::{is_on_path, read_output};

/// The compilers cmake can drive, in the order a configured one is looked for.
pub const C_COMPILERS: [&str; 5] = ["cl", "clang-cl", "clang", "gcc", "cc"];

/// Prefers Ninja when present, and on Windows loads MSVC through
/// `vcvarsall.bat` when no compiler is on `PATH`. A compiler that is already
/// set up is respected, never overridden.
pub fn configure() {
    // Ninja works with every compiler above and sidesteps cmake's
    // Visual-Studio-named-generator detection, which may not recognise a
    // newer VS version.
    if env::var_os("CMAKE_GENERATOR").is_none() && is_on_path("ninja") {
        set_var("CMAKE_GENERATOR", "Ninja");
    }
    if C_COMPILERS.iter().any(|compiler| is_on_path(compiler)) || !cfg!(windows) {
        return;
    }
    // Nothing found: MSVC is the most likely thing installed, even when it
    // is not on PATH, on a Windows machine with no compiler set up.
    if let Some(vcvarsall) = find_vcvarsall() {
        load_vcvarsall(&vcvarsall);
    }
}

/// The first `vcvarsall.bat` under either Program Files directory.
fn find_vcvarsall() -> Option<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(env::var_os)
        .map(|root| PathBuf::from(root).join("Microsoft Visual Studio"))
        .flat_map(subdirectories)
        .flat_map(subdirectories)
        .map(|edition| edition.join(r"VC\Auxiliary\Build\vcvarsall.bat"))
        .find(|candidate| candidate.is_file())
}

fn subdirectories(dir: PathBuf) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect()
}

/// Runs `vcvarsall.bat` for the host architecture and copies the
/// environment it produces into this process.
fn load_vcvarsall(vcvarsall: &std::path::Path) {
    let arch = match env::var("PROCESSOR_ARCHITECTURE").as_deref() {
        Ok("ARM64") => "arm64",
        _ => "x64",
    };
    println!(
        "==> No compiler on PATH; loading MSVC ({} {arch})...",
        vcvarsall.display()
    );
    let script = format!("\"{}\" {arch} >nul 2>&1 && set", vcvarsall.display());
    let Some(environment) = read_output(Command::new("cmd").args(["/c", &script])) else {
        println!("Note: vcvarsall.bat failed; continuing without it.");
        return;
    };
    for (name, value) in environment.lines().filter_map(|line| line.split_once('=')) {
        set_var(name, value);
    }
}

fn set_var(name: &str, value: &str) {
    // xtask is single-threaded, so nothing reads the environment concurrently.
    env::set_var(name, value);
}
