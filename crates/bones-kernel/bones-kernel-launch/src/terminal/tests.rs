use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use super::*;

/// A described machine: which programs are installed, which variables are
/// set, and where alternatives links point.
#[derive(Default)]
struct FakeProbe {
    installed: HashSet<&'static str>,
    variables: HashMap<&'static str, &'static str>,
    alternatives: HashMap<&'static str, &'static str>,
}

impl FakeProbe {
    fn with(installed: &[&'static str]) -> Self {
        Self {
            installed: installed.iter().copied().collect(),
            ..Self::default()
        }
    }
}

impl Probe for FakeProbe {
    fn find(&self, program: &str) -> bool {
        self.installed.contains(program)
    }
    fn variable(&self, name: &str) -> Option<String> {
        self.variables.get(name).map(|value| value.to_string())
    }
    fn alternative(&self, link: &str) -> Option<String> {
        self.alternatives.get(link).map(|value| value.to_string())
    }
}

fn args(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn linux_takes_the_first_installed_terminal_in_table_order() {
    let command = linux(
        Path::new("/repo"),
        None,
        &FakeProbe::with(&["xterm", "konsole"]),
    )
    .unwrap();

    assert_eq!(command.get_program(), "konsole");
    assert_eq!(args(&command), ["--workdir", "/repo"]);
}

#[test]
fn linux_prefers_terminal_over_the_table() {
    let mut probe = FakeProbe::with(&["gnome-terminal", "kitty"]);
    probe.variables.insert("TERMINAL", "/usr/bin/kitty");

    let command = linux(Path::new("/repo"), None, &probe).unwrap();

    assert_eq!(command.get_program(), "kitty");
}

#[test]
fn linux_follows_the_alternatives_link_before_the_table() {
    let mut probe = FakeProbe::with(&["gnome-terminal", "alacritty"]);
    probe
        .alternatives
        .insert("x-terminal-emulator", "/usr/bin/alacritty");

    let command = linux(Path::new("/repo"), None, &probe).unwrap();

    assert_eq!(command.get_program(), "alacritty");
}

#[test]
fn linux_runs_the_command_then_leaves_a_shell() {
    let command = linux(
        Path::new("/repo"),
        Some("claude"),
        &FakeProbe::with(&["gnome-terminal"]),
    )
    .unwrap();

    assert_eq!(
        args(&command),
        [
            "--working-directory",
            "/repo",
            "--",
            "sh",
            "-c",
            "claude; exec \"${SHELL:-sh}\""
        ]
    );
}

#[test]
fn linux_with_no_terminal_lists_every_terminal_tried() {
    let error = linux(Path::new("/repo"), None, &FakeProbe::default()).unwrap_err();

    for known in KNOWN {
        assert!(error.contains(known.program), "{error}");
    }
    assert!(error.contains("$TERMINAL"), "{error}");
}

#[test]
fn windows_prefers_windows_terminal() {
    let command = windows(
        Path::new(r"C:\repo"),
        Some("claude"),
        &FakeProbe::with(&["wt"]),
    );

    assert_eq!(command.get_program(), "wt");
    assert_eq!(args(&command), ["-d", r"C:\repo", "cmd", "/K", "claude"]);
}

#[test]
fn windows_falls_back_to_cmd() {
    let command = windows(Path::new(r"C:\repo"), None, &FakeProbe::default());

    assert_eq!(command.get_program(), "cmd");
    assert_eq!(args(&command), ["/C", "start", "", "/D", r"C:\repo", "cmd"]);
}

#[test]
fn quoting_survives_awkward_paths() {
    assert_eq!(sh_quote("/a b/it's"), r"'/a b/it'\''s'");
    assert_eq!(applescript_escape(r#"cd "x" \ y"#), r#"cd \"x\" \\ y"#);
}
