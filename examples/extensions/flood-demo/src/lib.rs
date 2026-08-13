use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, publish, subscribe, Level};

const PUBLISHES_PER_TICK: u32 = 64;

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe("core/tick");
        log(Level::Info, "publishes 64 messages on every tick");
    }

    fn on_tick(_dt: f32) {
        for sequence in 0..PUBLISHES_PER_TICK {
            publish("flood/message", &sequence.to_le_bytes());
        }
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

bones_wasm_sdk::export!(Component);
