use std::sync::{Arc, Mutex};

use bones_kernel_infra::bus::{Bus, Envelope};
use bones_kernel_infra::logging::{Level, LogSink};
use bones_messages::extension_control::Reload;
use bones_messages::{DecodeMessage, Message};

use super::*;
use crate::log_history::LogHistory;
use crate::server::route;
use crate::tests::{recorded, request, state};

/// A state with a bus, and what an endpoint subscribed to everything received.
fn with_bus() -> (State, Arc<Mutex<Vec<Envelope>>>) {
    let bus = Bus::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    let endpoint = bus.register("listener", move |e: &Envelope| {
        sink.lock().unwrap().push(e.clone());
    });
    endpoint.subscribe("*");
    let mut state = state();
    state.bus = Some(bus);
    (state, received)
}

#[test]
fn publish_puts_hex_on_any_topic_as_the_named_sender() {
    let (state, received) = with_bus();
    let response = route(
        &request(
            "POST /publish?topic=input/key-down&sender=platform HTTP/1.1",
            "0100",
        ),
        &state,
    );
    state.bus.as_ref().unwrap().dispatch();

    assert_eq!(response.status, "200 OK");
    let received = received.lock().unwrap();
    assert_eq!(received[0].topic, "input/key-down");
    assert_eq!(received[0].sender, "platform");
    assert_eq!(received[0].payload, [1, 0]);
}

#[test]
fn publish_rejects_a_missing_topic_and_bad_hex() {
    let (state, _) = with_bus();
    let no_topic = route(&request("POST /publish HTTP/1.1", "00"), &state);
    let odd = route(&request("POST /publish?topic=a/b HTTP/1.1", "abc"), &state);
    let not_hex = route(&request("POST /publish?topic=a/b HTTP/1.1", "zz"), &state);

    assert_eq!(no_topic.status, "400 Bad Request");
    assert_eq!(odd.status, "400 Bad Request");
    assert_eq!(not_hex.status, "400 Bad Request");
}

#[test]
fn messages_as_json_carry_the_whole_payload_as_hex() {
    let state = state();
    let mut message = recorded(3, "gfx/clear", "pong", "[4 bytes] ....");
    message.payload = Some(vec![0x11, 0x22, 0x33, 0xff]);
    state.messages.lock().unwrap().push(message);

    let response = route(&request("GET /messages?format=json HTTP/1.1", ""), &state);

    assert_eq!(response.content_type, "application/json");
    assert!(
        response.text_body().contains("\"hex\":\"112233ff\""),
        "{}",
        response.text_body()
    );
}

#[test]
fn extensions_lists_each_last_state() {
    let state = state();
    state
        .extensions
        .lock()
        .unwrap()
        .insert("pong".to_string(), "faulted".to_string());

    let response = route(&request("GET /extensions HTTP/1.1", ""), &state);

    assert_eq!(
        response.text_body().trim(),
        r#"[{"name":"pong","state":"faulted"}]"#
    );
}

#[test]
fn reload_publishes_the_runtime_command_as_devtools() {
    let (state, received) = with_bus();
    route(
        &request("POST /extensions/reload?name=pong HTTP/1.1", ""),
        &state,
    );
    state.bus.as_ref().unwrap().dispatch();

    let received = received.lock().unwrap();
    assert_eq!(received[0].topic, Reload::TOPIC);
    assert_eq!(received[0].sender, "devtools");
    assert_eq!(
        Reload::decode(&received[0].payload).unwrap().extension,
        "pong"
    );
}

#[test]
fn logs_filter_by_category_and_level_and_keep_the_newest() {
    let history = LogHistory::new();
    history.log(Level::Info, "engine", "one");
    history.log(Level::Warn, "pong", "two");
    history.log(Level::Warn, "pong", "three");
    let mut state = state();
    state.logs = Some(history);

    let response = route(
        &request("GET /logs?category=pong&level=warn&limit=1 HTTP/1.1", ""),
        &state,
    );

    assert_eq!(
        response.text_body().trim(),
        r#"[{"category":"pong","level":"warn","message":"three"}]"#
    );
}

#[test]
fn logs_say_so_when_the_composition_records_none() {
    let response = route(&request("GET /logs HTTP/1.1", ""), &state());
    assert_eq!(response.status, "404 Not Found");
}

#[test]
fn debug_lines_are_not_kept() {
    let history = LogHistory::new();
    history.log(Level::Debug, "runner", "tick");
    history.log(Level::Info, "engine", "loaded");
    assert_eq!(history.select(None, None, 10).len(), 1);
}

#[test]
fn exit_sets_the_exit_request() {
    let exit = bones_kernel_infra::exit::ExitRequest::new();
    let mut state = state();
    state.exit = Some(exit.clone());
    route(&request("POST /exit HTTP/1.1", ""), &state);
    assert!(exit.is_requested());
}

#[test]
fn a_send_answers_with_the_reply_the_module_made() {
    let (state, received) = with_bus();
    // Stands in for the module on the frame thread: answer the first send.
    let replies = state.replies.clone();
    let bus = state.bus.clone().unwrap();
    let answerer = std::thread::spawn(move || loop {
        bus.dispatch();
        if let Some(envelope) = received.lock().unwrap().first().cloned() {
            let send = crate::direct_send::DirectSend::decode(&envelope.payload).unwrap();
            replies
                .lock()
                .unwrap()
                .insert(send.id, Ok(b"\x2a".to_vec()));
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    });

    let response = route(
        &request("POST /send?endpoint=calculator HTTP/1.1", "7b7d"),
        &state,
    );

    answerer.join().unwrap();
    assert_eq!(response.text_body(), "2a\n");
}
