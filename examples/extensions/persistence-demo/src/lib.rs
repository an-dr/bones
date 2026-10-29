use std::cell::Cell;

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::{bus, Guest};

thread_local! {
    static COUNTER: Cell<u32> = const { Cell::new(0) };
    // Ticks arrive at the engine's tick rate (60Hz default) — saving on
    // every one would thrash the disk for no benefit; this throttles
    // actual writes to about once a second, same restraint keyecho
    // already applies to noisy topics like mouse-move.
    static ELAPSED: Cell<f32> = const { Cell::new(0.0) };
}

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        bus::subscribe::<Tick>();

        // The reply *is* the saved counter: a load has nothing to configure
        // beyond "whose", which the sender's own name decides. Any reply
        // that isn't exactly 4 bytes — never saved yet, or something else
        // entirely wrote to this slot — is treated as "start fresh" rather
        // than a hard error.
        let counter = match bus::load_state() {
            Ok(bytes) if bytes.len() == 4 => u32::from_le_bytes(bytes.try_into().unwrap()),
            _ => 0,
        };
        COUNTER.set(counter);
        log(
            Level::Info,
            &format!("init: loaded counter = {counter} (0 means nothing was saved yet)"),
        );
    }

    fn on_tick(dt: f32) {
        let elapsed = ELAPSED.get() + dt;
        if elapsed < 1.0 {
            ELAPSED.set(elapsed);
            return;
        }
        ELAPSED.set(0.0);

        let counter = COUNTER.get() + 1;
        COUNTER.set(counter);
        bus::save_state(&counter.to_le_bytes());
        log(Level::Info, &format!("tick: saved counter = {counter}"));
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

bones_wasm_sdk::export!(Component);
