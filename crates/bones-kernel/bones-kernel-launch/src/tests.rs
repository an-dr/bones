use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;

fn program(name: &str, cwd: PathBuf) -> Program {
    Program {
        program: name.into(),
        args: Vec::new(),
        cwd,
        env: Vec::new(),
    }
}

#[test]
fn an_unknown_program_is_not_found() {
    assert_eq!(resolve("bones-kernel-launch-no-such-program"), None);
    assert_eq!(resolve("   "), None);
    assert_eq!(resolve("~/bones-kernel-launch-no-such/program"), None);
}

#[test]
fn an_explicit_path_is_found_as_given() {
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::write(&tool, b"").unwrap();

    assert_eq!(resolve(&tool.to_string_lossy()), Some(tool));
}

#[test]
#[cfg(windows)]
fn a_bare_name_on_windows_also_matches_its_pathext_extensions() {
    let dir = tempfile::tempdir().unwrap();
    let shim = dir.path().join("tool.cmd");
    std::fs::write(&shim, b"@echo off").unwrap();

    // Asked for without an extension, as a user configures `code`.
    let found = resolve(&dir.path().join("tool").to_string_lossy());

    assert_eq!(found, Some(shim));
}

#[test]
#[cfg(windows)]
fn a_bare_name_on_windows_prefers_its_pathext_shim_over_an_extensionless_file() {
    let dir = tempfile::tempdir().unwrap();
    // VS Code's bin folder: a shell script `code` beside `code.cmd`.
    std::fs::write(dir.path().join("tool"), b"#!/bin/sh").unwrap();
    let shim = dir.path().join("tool.cmd");
    std::fs::write(&shim, b"@echo off").unwrap();

    let found = resolve(&dir.path().join("tool").to_string_lossy());

    assert_eq!(found, Some(shim));
}

#[test]
fn a_leading_tilde_is_the_home_folder() {
    let home = dirs::home_dir().unwrap();
    let marker = format!("bones-kernel-launch-tilde-{}", std::process::id());
    std::fs::write(home.join(&marker), b"").unwrap();

    let found = resolve(&format!("~/{marker}"));

    std::fs::remove_file(home.join(&marker)).unwrap();
    assert_eq!(found, Some(home.join(&marker)));
}

#[test]
fn launching_an_unknown_program_reports_what_was_searched() {
    let cwd = std::env::temp_dir();

    let outcome = launch(&program("bones-kernel-launch-no-such-program", cwd)).unwrap();

    assert_eq!(
        outcome,
        Launched::NotFound {
            searched: "bones-kernel-launch-no-such-program".into()
        }
    );
}

/// The program runs in the given folder with the given variable: it writes a
/// marker file there, which is how the test sees it ran at all.
#[test]
#[cfg(windows)]
fn a_cmd_shim_runs_in_its_folder_with_its_environment() {
    let dir = tempfile::tempdir().unwrap();
    let shim = dir.path().join("shim.cmd");
    std::fs::write(&shim, b"@echo %BONES_LAUNCH_TEST%> marker.txt\r\n").unwrap();
    let mut shim_program = program(&shim.to_string_lossy(), dir.path().to_path_buf());
    shim_program.env = vec![("BONES_LAUNCH_TEST".into(), "from-env".into())];

    assert_eq!(launch(&shim_program).unwrap(), Launched::Started);

    let marker = dir.path().join("marker.txt");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.is_file() || std::fs::read(&marker).unwrap().is_empty() {
        assert!(Instant::now() < deadline, "the shim never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        String::from_utf8(std::fs::read(&marker).unwrap())
            .unwrap()
            .trim(),
        "from-env"
    );
}

#[test]
#[cfg(unix)]
fn a_program_runs_in_its_folder_with_its_environment() {
    let dir = tempfile::tempdir().unwrap();
    let mut sh = program("sh", dir.path().to_path_buf());
    sh.args = vec![
        "-c".into(),
        "echo \"$BONES_LAUNCH_TEST\" > marker.txt".into(),
    ];
    sh.env = vec![("BONES_LAUNCH_TEST".into(), "from-env".into())];

    assert_eq!(launch(&sh).unwrap(), Launched::Started);

    let marker = dir.path().join("marker.txt");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.is_file() || std::fs::read(&marker).unwrap().is_empty() {
        assert!(Instant::now() < deadline, "the program never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        String::from_utf8(std::fs::read(&marker).unwrap())
            .unwrap()
            .trim(),
        "from-env"
    );
}
