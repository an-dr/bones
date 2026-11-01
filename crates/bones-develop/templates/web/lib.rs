//! An HTML page in a web panel that talks to this extension in JSON.

use std::cell::Cell;

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::web::{Command, OpenPanel, PageMessage, PanelSource, SendJson};
use bones_wasm_sdk::{bus, Guest, Inbound};

const PANEL: &str = "main";
const PAGE: &str = r#"<!doctype html>
<html><body style="font-family: sans-serif; background: #14141e; color: #eee">
  <h1 id="count">Clicked 0 times</h1>
  <button onclick="window.ipc.postMessage(JSON.stringify({ click: true }))">Click</button>
  <script>
    // The extension's SendJson arrives as a "bones-message" event.
    window.addEventListener("bones-message", (event) => {
      const message = JSON.parse(event.detail);
      document.getElementById("count").textContent = "Clicked " + message.count + " times";
    });
  </script>
</body></html>"#;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<PageMessage>();
        let opened = bus::web(&Command::Open(OpenPanel {
            panel: PANEL,
            source: PanelSource::Html(PAGE),
        }));
        if let Err(error) = opened {
            log(
                Level::Error,
                &format!("no web panels in this engine ({error:?}); run a -web build with web = true"),
            );
        }
    }

    fn shutdown() {}

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        if let Ok(Inbound::PageMessage(message)) = Inbound::decode(&topic, &payload) {
            if message.panel == PANEL && message.json.contains("\"click\"") {
                COUNT.set(COUNT.get() + 1);
                let json = format!("{{\"count\":{}}}", COUNT.get());
                let _ = bus::web(&Command::SendJson(SendJson { panel: PANEL, json: &json }));
            }
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
