//! The listener and the routes.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bones_kernel_infra::bus::{Bus, Envelope};
use bones_messages::web::PageMessage;
use bones_messages::{EncodeMessage, Message};

use crate::request::Request;

/// How long a connection may stay silent before it is dropped.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// Ceiling on a slow write of a response.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// What the routes need: the bus to post to, the recording to read, and the
/// flag that stops the listener.
#[derive(Clone)]
pub(crate) struct State {
    pub(crate) bus: Option<Bus>,
    pub(crate) messages: Arc<Mutex<Vec<String>>>,
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
    let (status, body) = route(&request, state);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

pub(crate) fn route(request: &Request, state: &State) -> (&'static str, String) {
    let mut parts = request.line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/health") => ("200 OK", "ok\n".to_string()),
        ("GET", "/messages") => {
            let messages = state.messages.lock().unwrap();
            let topic = parameter(query, "topic");
            let mut selected: Vec<&String> = messages
                .iter()
                .filter(|line| match &topic {
                    // The topic is the second tab-separated field.
                    Some(prefix) => line
                        .split('\t')
                        .nth(1)
                        .is_some_and(|candidate| candidate.starts_with(prefix.as_str())),
                    None => true,
                })
                .collect();
            if let Some(limit) = parameter(query, "limit").and_then(|l| l.parse::<usize>().ok()) {
                selected = selected.split_off(selected.len().saturating_sub(limit));
            }
            let body: Vec<&str> = selected.iter().map(|line| line.as_str()).collect();
            ("200 OK", format!("{}\n", body.join("\n")))
        }
        ("POST", "/messages/clear") => {
            state.messages.lock().unwrap().clear();
            ("200 OK", "cleared\n".to_string())
        }
        ("POST", "/page-message") => post_page_message(query, &request.body, state),
        _ => (
            "404 Not Found",
            "routes: GET /health, GET /messages, POST /messages/clear, POST /page-message?owner=&panel=\n"
                .to_string(),
        ),
    }
}

/// Publishes the request body as if the named panel's page had posted it.
fn post_page_message(query: &str, body: &str, state: &State) -> (&'static str, String) {
    let (Some(owner), Some(panel)) = (parameter(query, "owner"), parameter(query, "panel")) else {
        return (
            "400 Bad Request",
            "owner and panel are required\n".to_string(),
        );
    };
    let json = body.trim();
    if json.is_empty() {
        return ("400 Bad Request", "empty message body\n".to_string());
    }
    if serde_json::from_str::<serde_json::Value>(json).is_err() {
        return ("400 Bad Request", "invalid JSON message\n".into());
    }
    let Some(bus) = &state.bus else {
        return ("503 Service Unavailable", "no bus\n".to_string());
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
    ("200 OK", format!("posted to {owner}/{panel}\n"))
}

pub(crate) fn parameter(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}
