//! A counter with a button, as an egui widget panel.

use std::cell::Cell;

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::messages::ui::{Clicked, Spec, Widget};
use bones_wasm_sdk::{bus, Guest, Inbound};

const BUTTON: u32 = 1;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<Tick>();
        bus::subscribe::<Clicked>();
        log(Level::Info, "ready");
    }

    fn shutdown() {}

    /// Republishes the panel every frame; the ui module draws the latest.
    fn on_tick(_dt: f32) {
        let label = format!("Clicked {} times", COUNT.get());
        bus::publish(&Spec {
            title: "counter",
            widgets: vec![
                Widget::Label { text: &label },
                Widget::Button {
                    id: BUTTON,
                    label: "Click",
                },
            ],
        });
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match Inbound::decode(&topic, &payload) {
            Ok(Inbound::Clicked(clicked)) if clicked.id == BUTTON => COUNT.set(COUNT.get() + 1),
            Ok(_) => {}
            Err(error) => log(Level::Warn, &format!("bad {topic}: {error}")),
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
