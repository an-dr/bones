use bones_wasm_sdk::bindings::bones::extension::host_api::{log, publish, Level};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::messages::window::CloseRequested;
use bones_wasm_sdk::{bus, Guest};

struct Component;

impl Guest for Component {
    fn shutdown() {
        publish("hello/cleanup", b"complete");
        log(Level::Info, "shutdown");
    }

    fn init() {
        bus::subscribe::<Tick>();
        bus::subscribe::<CloseRequested>();
        log(Level::Info, "init");
    }

    fn on_tick(dt: f32) {
        log(Level::Debug, &format!("tick dt={dt}"));
    }

    fn on_message(topic: String, sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        log(Level::Debug, &format!("message on {topic} from {sender}"));
        publish("hello/received", topic.as_bytes());
        None
    }
}

bones_wasm_sdk::export!(Component);
