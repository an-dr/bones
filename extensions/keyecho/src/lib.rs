wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, subscribe, Level};

struct Component;

impl Guest for Component {
    fn init() {
        subscribe("input/key-down");
        log(Level::Info, "keyecho extension: init");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        if topic == "input/key-down" {
            let key = String::from_utf8_lossy(&payload);
            log(Level::Info, &format!("key pressed: {key}"));
        }
        None
    }
}

export!(Component);
