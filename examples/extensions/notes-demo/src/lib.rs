use std::cell::RefCell;

use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, publish, subscribe, Level};
use bones_wasm_sdk::messages::ui::{Changed, Clicked, Spec, Widget};
use bones_wasm_sdk::messages::{DecodeMessage, EncodeMessage, Message};

const INPUT_ID: u32 = 1;
const ADD_ID: u32 = 2;

// `State` stays with `Component` rather than splitting further: it's purely
// this extension's own thread-local store, never constructed or named
// outside this file, never meaningful on its own.
#[derive(Default)]
struct State {
    input: String,
    notes: Vec<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe("core/tick");
        subscribe(Clicked::TOPIC);
        subscribe(Changed::TOPIC);
        log(Level::Info, "init");
    }

    fn on_tick(_dt: f32) {
        STATE.with(|state| {
            let state = state.borrow();
            let mut widgets = vec![
                Widget::TextEdit {
                    id: INPUT_ID,
                    text: &state.input,
                },
                Widget::Button {
                    id: ADD_ID,
                    label: "Add",
                },
            ];
            for note in &state.notes {
                widgets.push(Widget::Label { text: note });
            }
            let spec = Spec {
                title: "notes",
                widgets,
            };
            publish(Spec::TOPIC, &spec.encode());
        });
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if topic == Changed::TOPIC {
                match Changed::decode(&payload) {
                    Ok(changed) if changed.id == INPUT_ID => state.input = changed.text.to_string(),
                    Ok(_) => {}
                    Err(err) => log(Level::Error, &format!("invalid ui/changed: {err}")),
                }
            } else if topic == Clicked::TOPIC {
                match Clicked::decode(&payload) {
                    Ok(clicked) if clicked.id == ADD_ID => {
                        if !state.input.is_empty() {
                            let note = std::mem::take(&mut state.input);
                            state.notes.push(note);
                        }
                    }
                    Ok(_) => {}
                    Err(err) => log(Level::Error, &format!("invalid ui/clicked: {err}")),
                }
            }
        });
        None
    }
}

bones_wasm_sdk::export!(Component);
