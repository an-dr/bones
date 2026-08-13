use std::cell::{Cell, RefCell};

use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, request_exit, send, subscribe, Level};
use bones_wasm_sdk::messages::web::{
    ClosePanel, Command, OpenPanel, PageMessage, PanelClosed, PanelFailed, PanelOpened,
    PanelSource, SendJson, ENDPOINT,
};
use bones_wasm_sdk::messages::{DecodeMessage, Message};

/// This extension's own name, as the host stamps it — the stem of the built
/// `.wasm`, so it tracks the crate name rather than the directory.
const OWNER: &str = "dashboard_demo";
const PANEL: &str = "main";
const UPDATE_TOPIC: &str = "metrics/updated";
const PAGE: &str = include_str!("../dashboard.html");

thread_local! {
    static PANEL_OPEN: Cell<bool> = const { Cell::new(false) };
    static PAGE_READY: Cell<bool> = const { Cell::new(false) };
    static LATEST_UPDATE: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct Component;

impl Guest for Component {
    fn shutdown() {
        if PANEL_OPEN.get() {
            let _ = send_web(Command::Close(ClosePanel { panel: PANEL }));
        }
    }

    fn init() {
        subscribe("web/*");
        subscribe(UPDATE_TOPIC);

        if send_web(Command::Open(OpenPanel {
            panel: PANEL,
            source: PanelSource::Html(PAGE),
        })) {
            PANEL_OPEN.set(true);
            log(Level::Info, "dashboard panel requested");
        }
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match topic.as_str() {
            UPDATE_TOPIC => receive_update(&payload),
            PageMessage::TOPIC => receive_page_message(&payload),
            PanelOpened::TOPIC => receive_opened(&payload),
            PanelClosed::TOPIC => receive_closed(&payload),
            PanelFailed::TOPIC => receive_failed(&payload),
            _ => {}
        }
        None
    }
}

fn receive_update(payload: &[u8]) {
    let Ok(json) = std::str::from_utf8(payload) else {
        log(Level::Warn, "ignored non-UTF-8 metrics update");
        return;
    };
    if serde_json::from_str::<serde_json::Value>(json).is_err() {
        log(Level::Warn, "ignored malformed metrics JSON");
        return;
    }
    LATEST_UPDATE.with(|latest| *latest.borrow_mut() = Some(json.to_string()));
    if PAGE_READY.get() {
        send_to_page(json);
    }
}

fn receive_page_message(payload: &[u8]) {
    let Ok(message) = PageMessage::decode(payload) else {
        return;
    };
    if message.owner != OWNER || message.panel != PANEL {
        return;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message.json) else {
        log(Level::Warn, "ignored malformed page JSON");
        return;
    };

    if value.get("ready").and_then(serde_json::Value::as_bool) == Some(true) {
        PAGE_READY.set(true);
        LATEST_UPDATE.with(|latest| {
            if let Some(json) = latest.borrow().as_deref() {
                send_to_page(json);
            }
        });
        log(Level::Info, "page ready");
    }
    if value.get("get").and_then(serde_json::Value::as_str) == Some("history") {
        let response = match send("metrics_demo", message.json.as_bytes()) {
            Ok(response) if serde_json::from_slice::<serde_json::Value>(&response).is_ok() => {
                response
            }
            Ok(_) => error_response("metrics returned an invalid reply"),
            Err(error) => error_response(&format!("metrics unavailable: {error:?}")),
        };
        send_to_page(std::str::from_utf8(&response).expect("error and validated JSON are UTF-8"));
    }
    if let Some(kind) = value.get("ack").and_then(serde_json::Value::as_str) {
        log(Level::Info, &format!("page acknowledged {kind}"));
    }
    if value.get("close").and_then(serde_json::Value::as_bool) == Some(true) {
        let _ = send_web(Command::Close(ClosePanel { panel: PANEL }));
        PANEL_OPEN.set(false);
        request_exit();
    }
}

fn receive_opened(payload: &[u8]) {
    let Ok(event) = PanelOpened::decode(payload) else {
        return;
    };
    if event.owner == OWNER && event.panel == PANEL {
        PANEL_OPEN.set(true);
        log(Level::Info, "panel opened");
    }
}

fn receive_closed(payload: &[u8]) {
    let Ok(event) = PanelClosed::decode(payload) else {
        return;
    };
    if event.owner == OWNER && event.panel == PANEL {
        PANEL_OPEN.set(false);
        log(Level::Info, "panel closed");
    }
}

fn receive_failed(payload: &[u8]) {
    let Ok(event) = PanelFailed::decode(payload) else {
        return;
    };
    if event.owner == OWNER && event.panel == PANEL {
        PANEL_OPEN.set(false);
        log(Level::Error, &format!("panel failed: {}", event.reason));
    }
}

fn send_to_page(json: &str) {
    let _ = send_web(Command::SendJson(SendJson { panel: PANEL, json }));
}

fn error_response(message: &str) -> Vec<u8> {
    serde_json::json!({
        "kind": "error",
        "message": message,
    })
    .to_string()
    .into_bytes()
}

fn send_web(command: Command<'_>) -> bool {
    match send(ENDPOINT, &command.encode()) {
        Ok(_) => true,
        Err(error) => {
            log(Level::Error, &format!("web command failed: {error:?}"));
            false
        }
    }
}

bones_wasm_sdk::export!(Component);
