//! The routes that hold, step, and look at the frame: `/pause`, `/resume`,
//! `/step`, `/frame`, and `/screenshot`.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::Envelope;
use bones_messages::renderer::{Capture, Scope};
use bones_messages::{EncodeMessage, Message};

use crate::devtools::ENDPOINT;
use crate::frames;
use crate::response::Response;
use crate::server::{parameter, State};

/// How long `/step` and `/screenshot` wait for the frame loop.
const WAIT: Duration = Duration::from_secs(5);
/// How often they look.
const POLL: Duration = Duration::from_millis(5);
/// The most frames one `/step` may ask for.
const MAX_STEPS: u32 = 10_000;

/// Holds the simulation still.
pub(crate) fn pause(state: &State) -> Response {
    match &state.frame_control {
        Some(control) => {
            control.pause();
            Response::text("200 OK", "paused\n")
        }
        None => no_frame_control(),
    }
}

/// Lets the simulation run again.
pub(crate) fn resume(state: &State) -> Response {
    match &state.frame_control {
        Some(control) => {
            control.resume();
            Response::text("200 OK", "running\n")
        }
        None => no_frame_control(),
    }
}

/// Runs `frames` frames while paused and answers once they have run.
pub(crate) fn step(query: &str, state: &State) -> Response {
    let Some(control) = &state.frame_control else {
        return no_frame_control();
    };
    if !control.is_paused() {
        return Response::text(
            "409 Conflict",
            "POST /pause first; stepping applies while paused\n",
        );
    }
    let frames = parameter(query, "frames")
        .and_then(|frames| frames.parse::<u32>().ok())
        .unwrap_or(1);
    if frames == 0 || frames > MAX_STEPS {
        return Response::text(
            "400 Bad Request",
            format!("frames must be between 1 and {MAX_STEPS}\n"),
        );
    }
    control.step(frames);
    let finished = wait_for(|| control.pending_steps() == 0);
    match finished {
        true => Response::text("200 OK", format!("stepped {frames} frames\n")),
        false => Response::text(
            "504 Gateway Timeout",
            format!(
                "{} of {frames} frames still pending; the frame loop is not running\n",
                control.pending_steps()
            ),
        ),
    }
}

/// The draw commands of the last frame.
pub(crate) fn frame(state: &State) -> Response {
    let batches = state.frames.lock().unwrap();
    Response::text("200 OK", frames::render(&batches))
}

/// The next presented frame as a PNG: the canvas, or with `scope=window`
/// the whole window, web panels included. The `X-Bones-Capture-Scope`
/// header says which one the image shows.
pub(crate) fn screenshot(query: &str, state: &State) -> Response {
    let Some(bus) = &state.bus else {
        return Response::text("503 Service Unavailable", "no bus\n");
    };
    let scope = match parameter(query, "scope").as_deref() {
        None | Some("canvas") => Scope::Canvas,
        Some("window") => Scope::Window,
        Some(other) => {
            return Response::text(
                "400 Bad Request",
                format!("scope is canvas or window, not '{other}'\n"),
            )
        }
    };
    let request_id = state.next_capture.fetch_add(1, Ordering::Relaxed);
    bus.publish(Envelope {
        topic: Capture::TOPIC.to_string(),
        sender: ENDPOINT.to_string(),
        correlation: None,
        payload: Capture { request_id, scope }.encode(),
    });
    let mut captured = None;
    wait_for(|| {
        captured = state.captures.lock().unwrap().remove(&request_id);
        captured.is_some()
    });
    match captured {
        Some((scope, png)) if !png.is_empty() => {
            let shows = match scope {
                Scope::Canvas => "canvas",
                Scope::Window => "window",
            };
            Response::png(png).with_header("X-Bones-Capture-Scope", shows)
        }
        Some(_) => Response::text(
            "500 Internal Server Error",
            "the renderer could not read the frame; GET /logs has its error\n",
        ),
        None => Response::text(
            "503 Service Unavailable",
            "no renderer answered; the composition has none, or the frame loop is not running\n",
        ),
    }
}

/// Polls `done` until it holds or [`WAIT`] passes; whether it held.
fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(POLL);
    }
    done()
}

fn no_frame_control() -> Response {
    Response::text(
        "404 Not Found",
        "this composition offers no frame-control service\n",
    )
}

#[cfg(test)]
mod tests;
