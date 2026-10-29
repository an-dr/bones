//! `bones-cli screenshot`: build the project, run its engine offscreen,
//! optionally press keys, step frames, and save what the renderer drew as
//! a PNG. One command for a documentation image or a design check, with no
//! display needed.
//!
//! With `--window` the engine runs in a visible window instead, and the
//! image is the whole window as shown, web panels included, where the
//! platform can capture it.

use std::path::PathBuf;

use serde_json::{json, Map};

use crate::catalogue::Catalogue;
use crate::dev::{self, Output, Window};
use crate::devtools_client::{DevtoolsClient, Reply};
use crate::project::Project;
use crate::{hex, Result};

const USAGE: &str = "usage: bones-cli screenshot [--out <file.png>] [--frames <n>] [--press <key>]... [--window] [--engine <path>]";

/// What to capture, from the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    out: PathBuf,
    frames: u32,
    presses: Vec<String>,
    /// The whole visible window rather than the offscreen canvas.
    window: bool,
    engine: Option<String>,
}

pub fn run(args: &[String]) -> Result {
    let options = parse(args)?;
    let project = Project::find()?;
    project.build()?;
    let client = DevtoolsClient {
        port: dev::free_port()?,
    };
    let port = client.port.to_string();
    let window = match options.window {
        true => Window::Visible,
        false => Window::Offscreen,
    };
    let mut child = dev::start(
        &project,
        options.engine.as_deref(),
        &port,
        Output::StderrOnly,
        window,
    )?;
    let taken = dev::wait_until_ready(&project, &client, &mut child)
        .and_then(|()| capture(&client, &options));
    dev::stop(&client, child);
    let (png, shows) = taken?;
    std::fs::write(&options.out, &png)
        .map_err(|error| format!("writing {}: {error}", options.out.display()))?;
    let (width, height) = png_size(&png).unwrap_or_default();
    println!(
        "wrote {} ({width}x{height}, {shows})",
        options.out.display()
    );
    if options.window && shows != "window" {
        println!("the window could not be captured here, so this is the canvas: no web panels; the engine log says why");
    }
    Ok(())
}

/// Pauses the engine, presses each key for one frame, steps the rest, and
/// returns the next presented frame as PNG bytes, with what it shows:
/// `canvas` or `window`.
fn capture(client: &DevtoolsClient, options: &Options) -> Result<(Vec<u8>, String)> {
    let catalogue = Catalogue::load();
    ok(client.request("POST", "/pause", b"")?)?;
    for key in &options.presses {
        let fields: Map<_, _> = [("key".to_string(), json!(key))].into_iter().collect();
        for topic in ["input/key-down", "input/key-up"] {
            let payload = catalogue.encode(topic, &fields)?;
            let target = format!("/publish?topic={topic}&sender=platform");
            ok(client.request("POST", &target, hex::format(&payload).as_bytes())?)?;
            ok(client.request("POST", "/step?frames=1", b"")?)?;
        }
    }
    ok(client.request("POST", &format!("/step?frames={}", options.frames), b"")?)?;
    let target = match options.window {
        true => "/screenshot?scope=window",
        false => "/screenshot",
    };
    let reply = ok(client.request("GET", target, b"")?)?;
    let shows = reply
        .header("x-bones-capture-scope")
        .unwrap_or("canvas")
        .to_string();
    match reply.content_type.as_str() {
        "image/png" => Ok((reply.body, shows)),
        other => Err(format!(
            "the engine answered {other}, not a PNG: {}",
            reply.text()
        )),
    }
}

fn ok(reply: Reply) -> Result<Reply> {
    match reply.status {
        200..=299 => Ok(reply),
        status => Err(format!(
            "devtools answered {status}: {}",
            reply.text().trim()
        )),
    }
}

fn parse(args: &[String]) -> Result<Options> {
    let mut options = Options {
        out: PathBuf::from("screenshot.png"),
        frames: 1,
        presses: Vec::new(),
        window: false,
        engine: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--window" {
            options.window = true;
            continue;
        }
        let value = args.next().ok_or_else(|| USAGE.to_string())?;
        match arg.as_str() {
            "--out" => options.out = PathBuf::from(value),
            "--frames" => {
                options.frames =
                    value
                        .parse()
                        .ok()
                        .filter(|frames| *frames > 0)
                        .ok_or_else(|| {
                            format!("--frames takes a whole number above 0, not '{value}'")
                        })?
            }
            "--press" => options.presses.push(value.clone()),
            "--engine" => options.engine = Some(value.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok(options)
}

/// Width and height from a PNG's header chunk.
fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    let width = png.get(16..20)?.try_into().ok()?;
    let height = png.get(20..24)?.try_into().ok()?;
    Some((u32::from_be_bytes(width), u32::from_be_bytes(height)))
}

#[cfg(test)]
mod tests;
