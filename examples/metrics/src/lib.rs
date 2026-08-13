use std::cell::{Cell, RefCell};

use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, publish, subscribe, Level};

const UPDATE_TOPIC: &str = "metrics/updated";
const UPDATE_INTERVAL: f32 = 0.5;
const HISTORY_LIMIT: usize = 12;

thread_local! {
    static ELAPSED: Cell<f32> = const { Cell::new(0.0) };
    static VALUE: Cell<u32> = const { Cell::new(0) };
    static HISTORY: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe("core/tick");
        publish_update(0);
        log(
            Level::Info,
            "ready: publishing metrics/updated twice per second",
        );
    }

    fn on_tick(dt: f32) {
        let elapsed = ELAPSED.get() + dt;
        if elapsed < UPDATE_INTERVAL {
            ELAPSED.set(elapsed);
            return;
        }
        ELAPSED.set(elapsed - UPDATE_INTERVAL);

        let value = VALUE.get() + 1;
        VALUE.set(value);
        HISTORY.with(|history| {
            let mut history = history.borrow_mut();
            history.push(value);
            if history.len() > HISTORY_LIMIT {
                history.remove(0);
            }
        });
        publish_update(value);
    }

    fn on_message(topic: String, sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        if !topic.is_empty() {
            return None;
        }
        let request: serde_json::Value = serde_json::from_slice(&payload).ok()?;
        if request.get("get")?.as_str()? != "history" {
            return None;
        }
        let id = request.get("id").and_then(serde_json::Value::as_u64);
        let history = HISTORY.with(|history| history.borrow().clone());
        let response = serde_json::json!({
            "kind": "history",
            "id": id,
            "values": history,
        });
        log(Level::Info, &format!("history requested by {sender}"));
        Some(response.to_string().into_bytes())
    }
}

fn publish_update(value: u32) {
    let payload = serde_json::json!({
        "kind": "update",
        "value": value,
    });
    publish(UPDATE_TOPIC, payload.to_string().as_bytes());
}

bones_wasm_sdk::export!(Component);
