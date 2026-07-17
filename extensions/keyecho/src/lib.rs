wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, subscribe, Level};
use bones_messages::input::KeyDown;
use bones_messages::{DecodeMessage, Message};

struct Component;

impl Guest for Component {
    fn init() {
        subscribe(KeyDown::TOPIC);
        log(Level::Info, "init");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        if topic == KeyDown::TOPIC {
            match KeyDown::decode(&payload) {
                Ok(message) => log(Level::Info, &format!("key pressed: {}", message.key)),
                Err(error) => log(Level::Error, &format!("invalid key event: {error}")),
            }
        }
        None
    }
}

export!(Component);
