wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, subscribe, Level};

struct Component;

impl Guest for Component {
    fn init() {
        subscribe("core/tick");
        log(Level::Info, "hello extension: init");
    }

    fn on_tick(dt: f32) {
        log(Level::Debug, &format!("hello extension: tick dt={dt}"));
    }

    fn on_message(topic: String, sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        log(Level::Debug, &format!("hello extension: message on {topic} from {sender}"));
        publish("hello/received", topic.as_bytes());
        None
    }
}

export!(Component);
