//! The listener and the routes.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bones_kernel_infra::bus::{Bus, Envelope};
use bones_kernel_infra::exit::ExitRequest;
use bones_kernel_infra::frame_control::FrameControl;
use bones_messages::renderer::Scope;
use bones_messages::web::PageMessage;
use bones_messages::{EncodeMessage, Message};

use crate::control;
use crate::frame_routes;
use crate::frames::Batches;
use crate::log_history::LogHistory;
use crate::recorded::Recorded;
use crate::request::Request;
use crate::response::Response;

/// How long a connection may stay silent before it is dropped.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// Ceiling on a slow write of a response.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Every route, for the 404 answer.
const ROUTES: &str = "routes: GET /health, POST /page-message?owner=&panel=, \
POST /publish?topic=[&sender=], GET /messages[?topic=&limit=&format=json], \
POST /messages/clear, GET /extensions, POST /extensions/{load,unload,reload}?name=, \
GET /logs[?category=&level=&limit=], POST /pause, POST /resume, POST /step?frames=, \
GET /frame, GET /screenshot[?scope=window], POST /send?endpoint=, POST /exit\n";

/// What the routes need: the bus to post to, what the module recorded, and
/// Captures the renderer answered, by request id, until `/screenshot`
/// collects them: what each shows, and its PNG.
pub(crate) type Captures = Arc<Mutex<HashMap<u32, (Scope, Vec<u8>)>>>;

/// the flag that stops the listener.
#[derive(Clone)]
pub(crate) struct State {
    pub(crate) bus: Option<Bus>,
    pub(crate) messages: Arc<Mutex<Vec<Recorded>>>,
    /// Each extension's last `core/lifecycle` state, by name.
    pub(crate) extensions: Arc<Mutex<BTreeMap<String, String>>>,
    /// The composition's log, when it was teed into the channel.
    pub(crate) logs: Option<LogHistory>,
    /// The `frame-control` service, when the composition offers it.
    pub(crate) frame_control: Option<FrameControl>,
    /// Each sender's latest `gfx/*` batch, for `/frame`.
    pub(crate) frames: Batches,
    /// `renderer/captured` PNGs not yet collected, by request id.
    pub(crate) captures: Captures,
    /// The next `renderer/capture` request id.
    pub(crate) next_capture: Arc<AtomicU32>,
    /// The application's exit request, when the composition offers it.
    pub(crate) exit: Option<ExitRequest>,
    /// `/send` replies the module made, by request id.
    pub(crate) replies: crate::direct_send::Replies,
    /// The next `/send` request id.
    pub(crate) next_send: Arc<AtomicU32>,
    pub(crate) shutdown: Arc<AtomicBool>,
}

/// Accepts until shutdown, answering each connection on a thread of its own
/// so one silent client cannot hold up the next.
pub(crate) fn serve(listener: TcpListener, state: State) {
    while !state.shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let state = state.clone();
                let _ = std::thread::Builder::new()
                    .name("bones-devtools-connection".to_string())
                    .spawn(move || answer(stream, &state));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

fn answer(mut stream: TcpStream, state: &State) {
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let _ = stream.set_read_timeout(Some(REQUEST_TIMEOUT));
    let _ = stream.set_write_timeout(Some(RESPONSE_TIMEOUT));
    let Some(request) = Request::read(&mut stream) else {
        return;
    };
    let response = route(&request, state);
    let extra: String = response
        .headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect();
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n{extra}Connection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&response.body);
}

pub(crate) fn route(request: &Request, state: &State) -> Response {
    let mut parts = request.line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/health") => Response::text("200 OK", "ok\n"),
        ("GET", "/messages") => messages(query, state),
        ("POST", "/messages/clear") => {
            state.messages.lock().unwrap().clear();
            Response::text("200 OK", "cleared\n")
        }
        ("POST", "/page-message") => post_page_message(query, &request.body, state),
        ("POST", "/publish") => control::publish(query, &request.body, state),
        ("GET", "/extensions") => control::extensions(state),
        ("POST", "/extensions/load") => control::command("load", query, state),
        ("POST", "/extensions/unload") => control::command("unload", query, state),
        ("POST", "/extensions/reload") => control::command("reload", query, state),
        ("GET", "/logs") => control::logs(query, state),
        ("POST", "/pause") => frame_routes::pause(state),
        ("POST", "/resume") => frame_routes::resume(state),
        ("POST", "/step") => frame_routes::step(query, state),
        ("GET", "/frame") => frame_routes::frame(state),
        ("GET", "/screenshot") => frame_routes::screenshot(query, state),
        ("POST", "/send") => control::send(query, &request.body, state),
        ("POST", "/exit") => control::exit(state),
        _ => Response::text("404 Not Found", ROUTES),
    }
}

/// The recording, filtered by topic prefix and cut to the newest `limit`.
fn messages(query: &str, state: &State) -> Response {
    let messages = state.messages.lock().unwrap();
    let topic = parameter(query, "topic");
    let mut selected: Vec<&Recorded> = messages
        .iter()
        .filter(|recorded| {
            topic
                .as_ref()
                .is_none_or(|prefix| recorded.topic.starts_with(prefix.as_str()))
        })
        .collect();
    if let Some(limit) = parameter(query, "limit").and_then(|l| l.parse::<usize>().ok()) {
        selected = selected.split_off(selected.len().saturating_sub(limit));
    }
    if parameter(query, "format").as_deref() == Some("json") {
        let list = selected.iter().map(|recorded| recorded.to_json()).collect();
        return Response::json(&serde_json::Value::Array(list));
    }
    let lines: Vec<String> = selected.iter().map(|recorded| recorded.line()).collect();
    Response::text("200 OK", format!("{}\n", lines.join("\n")))
}

/// Publishes the request body as if the named panel's page had posted it.
fn post_page_message(query: &str, body: &str, state: &State) -> Response {
    let (Some(owner), Some(panel)) = (parameter(query, "owner"), parameter(query, "panel")) else {
        return Response::text("400 Bad Request", "owner and panel are required\n");
    };
    let json = body.trim();
    if json.is_empty() {
        return Response::text("400 Bad Request", "empty message body\n");
    }
    if serde_json::from_str::<serde_json::Value>(json).is_err() {
        return Response::text("400 Bad Request", "invalid JSON message\n");
    }
    let Some(bus) = &state.bus else {
        return Response::text("503 Service Unavailable", "no bus\n");
    };
    bus.publish(Envelope {
        topic: PageMessage::TOPIC.to_string(),
        // Named as the web module, because that is who this stands in for:
        // an extension filtering on the sender must not be able to tell a
        // driven message from a clicked one.
        sender: "web".to_string(),
        correlation: None,
        payload: PageMessage {
            owner: &owner,
            panel: &panel,
            json,
        }
        .encode(),
    });
    Response::text("200 OK", format!("posted to {owner}/{panel}\n"))
}

pub(crate) fn parameter(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}
