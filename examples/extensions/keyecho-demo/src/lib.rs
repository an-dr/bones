use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::input::{
    GamepadButtonDown, GamepadButtonUp, GamepadConnected, GamepadDisconnected, KeyDown, MouseDown,
    MouseUp, MouseWheel,
};
use bones_wasm_sdk::{bus, Guest, Inbound};

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        bus::subscribe::<KeyDown>();
        bus::subscribe::<MouseDown>();
        bus::subscribe::<MouseUp>();
        bus::subscribe::<MouseWheel>();
        bus::subscribe::<GamepadConnected>();
        bus::subscribe::<GamepadDisconnected>();
        bus::subscribe::<GamepadButtonDown>();
        bus::subscribe::<GamepadButtonUp>();
        log(Level::Info, "init");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        // Not `MouseMove` or `GamepadAxis`: both fire continuously (every
        // pixel of travel / every tilt of a stick), which would flood this
        // demo's log — clicks, scroll, buttons, and connection lifecycle
        // are enough to prove events reach an extension at all.
        match Inbound::decode(&topic, &payload) {
            Ok(Inbound::KeyDown(message)) => {
                log(Level::Info, &format!("key pressed: {}", message.key))
            }
            Ok(Inbound::MouseDown(message)) => log(
                Level::Info,
                &format!(
                    "mouse button {} pressed at ({}, {})",
                    message.button, message.x, message.y
                ),
            ),
            Ok(Inbound::MouseUp(message)) => log(
                Level::Info,
                &format!(
                    "mouse button {} released at ({}, {})",
                    message.button, message.x, message.y
                ),
            ),
            Ok(Inbound::MouseWheel(message)) => log(
                Level::Info,
                &format!("mouse wheel ({}, {})", message.x, message.y),
            ),
            Ok(Inbound::GamepadConnected(message)) => {
                log(Level::Info, &format!("gamepad {} connected", message.id))
            }
            Ok(Inbound::GamepadDisconnected(message)) => {
                log(Level::Info, &format!("gamepad {} disconnected", message.id))
            }
            Ok(Inbound::GamepadButtonDown(message)) => log(
                Level::Info,
                &format!("gamepad {} button {} pressed", message.id, message.button),
            ),
            Ok(Inbound::GamepadButtonUp(message)) => log(
                Level::Info,
                &format!("gamepad {} button {} released", message.id, message.button),
            ),
            Ok(_) => {}
            Err(error) => log(Level::Error, &format!("invalid {topic} event: {error}")),
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
