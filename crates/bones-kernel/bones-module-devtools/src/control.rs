//! The routes that act on the engine rather than on a page: publish any
//! message, see and drive extensions, and read the log.

use bones_kernel_infra::bus::Envelope;
use bones_messages::extension_control::{Load, Reload, Unload};
use bones_messages::{EncodeMessage, Message};

use crate::devtools::ENDPOINT;
use crate::direct_send::DirectSend;
use crate::log_history;
use crate::recorded::hex;
use crate::response::Response;
use crate::server::{parameter, State};

/// Lines `/logs` returns when the request names no limit.
const DEFAULT_LOG_LINES: usize = 200;
/// How long `/send` waits for the frame loop to make the call.
const SEND_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Publishes a hex payload on any topic.
pub(crate) fn publish(query: &str, body: &str, state: &State) -> Response {
    let Some(topic) = parameter(query, "topic").filter(|topic| !topic.is_empty()) else {
        return Response::text("400 Bad Request", "topic is required\n");
    };
    let Some(payload) = parse_hex(body.trim()) else {
        return Response::text(
            "400 Bad Request",
            "the body must be the payload as hex, two digits per byte\n",
        );
    };
    let Some(bus) = &state.bus else {
        return Response::text("503 Service Unavailable", "no bus\n");
    };
    let sender = parameter(query, "sender").unwrap_or_else(|| ENDPOINT.to_string());
    let length = payload.len();
    bus.publish(Envelope {
        topic: topic.clone(),
        sender: sender.clone(),
        correlation: None,
        payload,
    });
    Response::text(
        "200 OK",
        format!("published {length} bytes on {topic} as {sender}\n"),
    )
}

/// Each extension's last lifecycle state.
pub(crate) fn extensions(state: &State) -> Response {
    let states = state.extensions.lock().unwrap();
    let list: Vec<serde_json::Value> = states
        .iter()
        .map(|(name, state)| serde_json::json!({ "name": name, "state": state }))
        .collect();
    Response::json(&serde_json::Value::Array(list))
}

/// Publishes a runtime load, unload, or reload command for one extension.
pub(crate) fn command(action: &str, query: &str, state: &State) -> Response {
    let Some(name) = parameter(query, "name").filter(|name| !name.is_empty()) else {
        return Response::text("400 Bad Request", "name is required\n");
    };
    let (topic, payload) = match action {
        "load" => (Load::TOPIC, Load { extension: &name }.encode()),
        "unload" => (Unload::TOPIC, Unload { extension: &name }.encode()),
        _ => (Reload::TOPIC, Reload { extension: &name }.encode()),
    };
    let Some(bus) = &state.bus else {
        return Response::text("503 Service Unavailable", "no bus\n");
    };
    bus.publish(Envelope {
        topic: topic.to_string(),
        sender: ENDPOINT.to_string(),
        correlation: None,
        payload,
    });
    Response::text(
        "200 OK",
        format!(
            "asked to {action} '{name}'; GET /extensions shows the outcome, GET /logs any refusal\n"
        ),
    )
}

/// The last log lines, filtered by category and level.
pub(crate) fn logs(query: &str, state: &State) -> Response {
    let Some(history) = &state.logs else {
        return Response::text(
            "404 Not Found",
            "this composition does not record its log for the channel\n",
        );
    };
    let limit = parameter(query, "limit")
        .and_then(|limit| limit.parse().ok())
        .unwrap_or(DEFAULT_LOG_LINES);
    let category = parameter(query, "category");
    let level = parameter(query, "level");
    let lines: Vec<serde_json::Value> = history
        .select(category.as_deref(), level.as_deref(), limit)
        .into_iter()
        .map(|(level, category, message)| {
            serde_json::json!({
                "level": log_history::name(level),
                "category": category,
                "message": message,
            })
        })
        .collect();
    Response::json(&serde_json::Value::Array(lines))
}

/// Makes a direct send of a hex payload to `endpoint` and answers with the
/// reply as hex. The module makes the call on the frame thread
/// (`direct_send`), so this waits for the next delivery.
pub(crate) fn send(query: &str, body: &str, state: &State) -> Response {
    let Some(endpoint) = parameter(query, "endpoint").filter(|name| !name.is_empty()) else {
        return Response::text("400 Bad Request", "endpoint is required\n");
    };
    let Some(payload) = parse_hex(body.trim()) else {
        return Response::text(
            "400 Bad Request",
            "the body must be the payload as hex, two digits per byte\n",
        );
    };
    let Some(bus) = &state.bus else {
        return Response::text("503 Service Unavailable", "no bus\n");
    };
    let id = state
        .next_send
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    bus.publish(
        DirectSend {
            id,
            endpoint: endpoint.clone(),
            payload,
        }
        .envelope(),
    );
    let deadline = std::time::Instant::now() + SEND_WAIT;
    while std::time::Instant::now() < deadline {
        if let Some(reply) = state.replies.lock().unwrap().remove(&id) {
            return match reply {
                Ok(bytes) => Response::text("200 OK", format!("{}\n", hex(&bytes))),
                Err(error) => Response::text(
                    "502 Bad Gateway",
                    format!("send to '{endpoint}' failed: {error}\n"),
                ),
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Response::text(
        "504 Gateway Timeout",
        "the frame loop did not make the send; is it running?\n",
    )
}

/// Asks the application to shut down in order.
pub(crate) fn exit(state: &State) -> Response {
    match &state.exit {
        Some(exit) => {
            exit.request();
            Response::text("200 OK", "exiting\n")
        }
        None => Response::text(
            "404 Not Found",
            "this composition offers no exit-request service\n",
        ),
    }
}

/// Bytes from hex digits; `None` for an odd length or a non-hex digit.
fn parse_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests;
