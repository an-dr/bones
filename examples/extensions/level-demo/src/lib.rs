use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::{bus, Guest};

// Edit this, rebuild (cargo xtask example level-demo), and watch the next log line change
// without restarting the app — that's the hot-reload demo (README.md).
const VERSION: &str = "v1";

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        bus::subscribe::<Tick>();
        log(Level::Info, &format!("{VERSION}: loaded"));
    }

    fn on_tick(_dt: f32) {}

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

bones_wasm_sdk::export!(Component);
