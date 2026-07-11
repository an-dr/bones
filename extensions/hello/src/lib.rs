wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, Level};

struct Component;

impl Guest for Component {
    fn init() {
        log(Level::Info, "hello extension: init");
    }

    fn on_tick(dt: f32) {
        log(Level::Debug, &format!("hello extension: tick dt={dt}"));
    }
}

export!(Component);
