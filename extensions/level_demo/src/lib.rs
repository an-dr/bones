wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, subscribe, Level};

// Edit this, rebuild (pwsh build.ps1), and watch the next log line change
// without restarting the app — that's the hot-reload demo (README.md).
const VERSION: &str = "v1";

struct Component;

impl Guest for Component {
    fn init() {
        subscribe("core/tick");
        log(Level::Info, &format!("{VERSION}: loaded"));
    }

    fn on_tick(_dt: f32) {}

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

export!(Component);
