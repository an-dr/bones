wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::Cell;

use bones::core::host_api::{log, publish, send, subscribe, Level};
use bones_messages::persistence::{Save, ENDPOINT};
use bones_messages::{EncodeMessage, Message};

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
        subscribe("core/tick");

        // Empty payload: `send`'s reply *is* the saved counter, nothing to
        // configure about a load beyond "whose" (persistence/*'s own doc
        // comment). Any reply that isn't exactly 4 bytes — never saved
        // yet, or something else entirely wrote to this slot — is treated
        // as "start fresh" rather than a hard error.
        let counter = match send(ENDPOINT, &[]) {
            Ok(bytes) if bytes.len() == 4 => u32::from_le_bytes(bytes.try_into().unwrap()),
            _ => 0,
        };
        COUNTER.set(counter);
        log(Level::Info, &format!("init: loaded counter = {counter} (0 means nothing was saved yet)"));
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
        publish(Save::TOPIC, &Save { bytes: &counter.to_le_bytes() }.encode());
        log(Level::Info, &format!("tick: saved counter = {counter}"));
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

export!(Component);
