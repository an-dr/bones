use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, subscribe, Level};

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe("core/tick");
        log(Level::Info, "init, about to hang on its first tick");
    }

    fn on_tick(_dt: f32) {
        // Deliberately never returns — proves the host's time budget
        // (ADR-007) traps and quarantines it instead of stalling the
        // engine forever. black_box keeps the optimizer from proving this
        // loop has no observable effect and eliminating it.
        let mut i: u64 = 0;
        loop {
            i = i.wrapping_add(1);
            std::hint::black_box(i);
        }
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

bones_wasm_sdk::export!(Component);
