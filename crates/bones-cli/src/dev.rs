//! `bones-cli dev`: build, run the engine, and rebuild on every change; the
//! engine hot-reloads the new `.wasm` (design/extensions.md).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};

use crate::devtools_client::DevtoolsClient;
use crate::engine;
use crate::project::Project;
use crate::Result;

const USAGE: &str = "usage: bones-cli dev [--engine <path>] [--devtools <port>]";
const DEFAULT_DEVTOOLS_PORT: &str = "8642";
/// How often the sources are looked at.
const POLL: Duration = Duration::from_millis(300);

pub fn run(args: &[String]) -> Result {
    let (engine_path, port) = parse(args)?;
    let project = Project::find()?;
    project.build()?;
    let mut child = start(
        &project,
        engine_path.as_deref(),
        &port,
        Output::Terminal,
        Window::Visible,
    )?;
    println!("devtools: http://127.0.0.1:{port} (bones-cli mcp serves it to an agent)");
    println!(
        "watching {} for changes; close the window to stop",
        project.root.join("extension").display()
    );

    let mut seen = snapshot(&project.root.join("extension"));
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return match status.success() {
                true => Ok(()),
                false => Err(format!("the engine exited with {status}")),
            };
        }
        std::thread::sleep(POLL);
        let now = snapshot(&project.root.join("extension"));
        if now != seen {
            seen = now;
            // A failed build leaves the running extension as it was; the
            // error is already on the terminal.
            if let Err(error) = project.build() {
                eprintln!("{error}");
            }
        }
    }
}

fn parse(args: &[String]) -> Result<(Option<String>, String)> {
    let mut engine = None;
    let mut port = DEFAULT_DEVTOOLS_PORT.to_string();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let value = args.next().ok_or_else(|| USAGE.to_string())?;
        match arg.as_str() {
            "--engine" => engine = Some(value.clone()),
            "--devtools" => port = value.clone(),
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok((engine, port))
}

/// Where a started engine's own output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// Inherited, so `dev` streams the log to the terminal.
    Terminal,
    /// Stdout redirected to stderr, for `mcp`, whose stdout is the protocol.
    StderrOnly,
}

/// Where a started engine draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// On the display, for a person to watch.
    Visible,
    /// In memory (`BONES_OFFSCREEN`): nothing on screen, no display needed,
    /// and `/screenshot` still answers.
    Offscreen,
}

/// Starts the shipped engine on the project's `bones.toml`, or the
/// project's own app when it has one; both with the devtools channel open.
pub fn start(
    project: &Project,
    engine_path: Option<&str>,
    port: &str,
    output: Output,
    window: Window,
) -> Result<Child> {
    ensure_port_free(port)?;
    let mut command = match project.has_app() {
        true => {
            let mut command = Command::new("cargo");
            command
                .current_dir(&project.root)
                .args(["run", "--release", "-p"])
                .arg(format!("{}-app", project.name));
            command
        }
        false => {
            let mut command = Command::new(engine::locate(engine_path)?);
            command.env("BONES_CONFIG", project.root.join("bones.toml"));
            command
        }
    };
    if output == Output::StderrOnly {
        command.stdout(std::io::stderr());
    }
    if window == Window::Offscreen {
        command.env("BONES_OFFSCREEN", "1");
    }
    command
        .env("BONES_DEVTOOLS_PORT", port)
        .spawn()
        .map_err(|error| format!("starting the engine: {error}"))
}

/// Waits until the engine `start`ed answers on its devtools channel. A
/// project with its own app is compiled first, so it is given longer.
pub fn wait_until_ready(project: &Project, client: &DevtoolsClient, child: &mut Child) -> Result {
    let wait = match project.has_app() {
        true => Duration::from_secs(300),
        false => Duration::from_secs(30),
    };
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if client.request("GET", "/health", b"").is_ok() {
            return Ok(());
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!(
                "the engine exited with {status} before it was ready"
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(
        "the engine started but its devtools channel never answered; check logs with bones-cli dev"
            .to_string(),
    )
}

/// Stops an engine `start`ed: an orderly exit first, so extensions run
/// their `shutdown`, then a kill if it hangs.
pub fn stop(client: &DevtoolsClient, mut child: Child) {
    let _ = client.request("POST", "/exit", b"");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// A port on `127.0.0.1` that nothing listens on right now.
pub fn free_port() -> Result<u16> {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("finding a free port: {error}"))
}

/// Fails with the fix when the devtools port is taken, usually by another
/// engine: the new one would exit, and a client would talk to the old one.
fn ensure_port_free(port: &str) -> Result {
    let number: u16 = port
        .parse()
        .map_err(|_| format!("'{port}' is not a port number"))?;
    std::net::TcpListener::bind(("127.0.0.1", number))
        .map(drop)
        .map_err(|_| {
            format!(
                "the devtools port {number} is already in use, probably by another engine; \
                 choose another with --devtools <port> (BONES_DEVTOOLS_PORT for bones-cli mcp)"
            )
        })
}

/// Modification times of every file under `dir`, skipping build output.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, SystemTime> {
    let mut times = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return times;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "target") {
            continue;
        }
        if path.is_dir() {
            times.extend(snapshot(&path));
        } else if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
            times.insert(path, modified);
        }
    }
    times
}

#[cfg(test)]
mod tests;
