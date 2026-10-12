//! Opening a terminal at a folder, optionally running a command in it.
//!
//! Every platform and every terminal spells this differently, so this is a
//! table of known spellings rather than a general mechanism. When nothing in
//! the table is installed the error lists what was tried instead of guessing.

use std::path::Path;
use std::process::Command;

mod known_terminal;
mod probe;

use known_terminal::{KnownTerminal, KNOWN};
use probe::{Probe, SystemProbe};

use crate::launch::spawn_detached;

/// Opens a terminal in `dir`, detached. With `run`, the terminal runs that
/// command and stays open afterwards, so its output is still there to read.
///
/// - Windows: Windows Terminal (`wt`) when installed, otherwise `cmd`.
/// - Linux: `$TERMINAL`, then whatever `x-terminal-emulator` points at, then
///   the first installed terminal from a table of known ones.
/// - macOS: Terminal.app.
pub fn open_terminal(dir: &Path, run: Option<&str>) -> Result<(), String> {
    let command = terminal_command(dir, run, &SystemProbe)?;
    spawn_detached(command).map_err(|error| format!("could not open a terminal: {error}"))
}

fn terminal_command(dir: &Path, run: Option<&str>, probe: &dyn Probe) -> Result<Command, String> {
    if cfg!(windows) {
        Ok(windows(dir, run, probe))
    } else if cfg!(target_os = "macos") {
        Ok(macos(dir, run))
    } else {
        linux(dir, run, probe)
    }
}

fn windows(dir: &Path, run: Option<&str>, probe: &dyn Probe) -> Command {
    let mut command;
    if probe.find("wt") {
        command = Command::new("wt");
        command.arg("-d").arg(dir);
        if let Some(run) = run {
            command.args(["cmd", "/K", run]);
        }
    } else {
        command = Command::new("cmd");
        command.args(["/C", "start", "", "/D"]).arg(dir).arg("cmd");
        if let Some(run) = run {
            command.args(["/K", run]);
        }
    }
    command
}

fn macos(dir: &Path, run: Option<&str>) -> Command {
    let mut command;
    match run {
        Some(run) => {
            let script = format!("cd {} && {run}", sh_quote(&dir.to_string_lossy()));
            command = Command::new("osascript");
            command
                .arg("-e")
                .arg(format!(
                    "tell application \"Terminal\" to do script \"{}\"",
                    applescript_escape(&script)
                ))
                .args(["-e", "tell application \"Terminal\" to activate"]);
        }
        None => {
            command = Command::new("open");
            command.args(["-a", "Terminal"]).arg(dir);
        }
    }
    command
}

fn linux(dir: &Path, run: Option<&str>, probe: &dyn Probe) -> Result<Command, String> {
    let preferred = probe.variable("TERMINAL").and_then(|name| basename(&name));
    let alternative = probe
        .alternative("x-terminal-emulator")
        .and_then(|target| basename(&target));
    let candidates: Vec<&KnownTerminal> = preferred
        .iter()
        .chain(alternative.iter())
        .map(String::as_str)
        .chain(KNOWN.iter().map(|known| known.program))
        .filter_map(|name| KNOWN.iter().find(|known| known.program == name))
        .collect();
    let Some(known) = candidates.iter().find(|known| probe.find(known.program)) else {
        let tried: Vec<&str> = KNOWN.iter().map(|known| known.program).collect();
        return Err(format!(
            "no supported terminal emulator was found (tried {}); set $TERMINAL",
            tried.join(", ")
        ));
    };

    let mut command = Command::new(known.program);
    command.current_dir(dir);
    if !known.dir_flag.is_empty() {
        command.args(known.dir_flag).arg(dir);
    }
    if let Some(run) = run {
        // Leave the user in a shell after the command exits, so the
        // terminal does not vanish with its output.
        command.args(known.exec_flag).args([
            "sh",
            "-c",
            &format!("{run}; exec \"${{SHELL:-sh}}\""),
        ]);
    }
    Ok(command)
}

fn basename(path: &str) -> Option<String> {
    Path::new(path.trim())
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn sh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn applescript_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests;
