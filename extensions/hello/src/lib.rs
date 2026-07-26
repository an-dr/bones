wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::window::CloseRequested;
use bones_messages::Message;

struct Component;

impl Guest for Component {
    fn shutdown() {
        publish("hello/cleanup", b"complete");
        log(Level::Info, "shutdown");
    }

    fn init() {
        subscribe("core/tick");
        subscribe(CloseRequested::TOPIC);
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

export!(Component);
