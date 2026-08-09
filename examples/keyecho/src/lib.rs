wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, subscribe, Level};
use bones_messages::input::{
    GamepadButtonDown, GamepadButtonUp, GamepadConnected, GamepadDisconnected, KeyDown, MouseDown,
    MouseUp, MouseWheel,
};
use bones_messages::{DecodeMessage, Message};

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe(KeyDown::TOPIC);
        subscribe(MouseDown::TOPIC);
        subscribe(MouseUp::TOPIC);
        subscribe(MouseWheel::TOPIC);
        subscribe(GamepadConnected::TOPIC);
        subscribe(GamepadDisconnected::TOPIC);
        subscribe(GamepadButtonDown::TOPIC);
        subscribe(GamepadButtonUp::TOPIC);
        log(Level::Info, "init");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        // Not `MouseMove` or `GamepadAxis`: both fire continuously (every
        // pixel of travel / every tilt of a stick), which would flood this
        // demo's log — clicks, scroll, buttons, and connection lifecycle
        // are enough to prove events reach an extension at all.
        match topic.as_str() {
            KeyDown::TOPIC => match KeyDown::decode(&payload) {
                Ok(message) => log(Level::Info, &format!("key pressed: {}", message.key)),
                Err(error) => log(Level::Error, &format!("invalid key event: {error}")),
            },
            MouseDown::TOPIC => match MouseDown::decode(&payload) {
                Ok(message) => log(
                    Level::Info,
                    &format!("mouse button {} pressed at ({}, {})", message.button, message.x, message.y),
                ),
                Err(error) => log(Level::Error, &format!("invalid mouse-down event: {error}")),
            },
            MouseUp::TOPIC => match MouseUp::decode(&payload) {
                Ok(message) => log(
                    Level::Info,
                    &format!("mouse button {} released at ({}, {})", message.button, message.x, message.y),
                ),
                Err(error) => log(Level::Error, &format!("invalid mouse-up event: {error}")),
            },
            MouseWheel::TOPIC => match MouseWheel::decode(&payload) {
                Ok(message) => log(Level::Info, &format!("mouse wheel ({}, {})", message.x, message.y)),
                Err(error) => log(Level::Error, &format!("invalid mouse-wheel event: {error}")),
            },
            GamepadConnected::TOPIC => match GamepadConnected::decode(&payload) {
                Ok(message) => log(Level::Info, &format!("gamepad {} connected", message.id)),
                Err(error) => log(Level::Error, &format!("invalid gamepad-connected event: {error}")),
            },
            GamepadDisconnected::TOPIC => match GamepadDisconnected::decode(&payload) {
                Ok(message) => log(Level::Info, &format!("gamepad {} disconnected", message.id)),
                Err(error) => log(Level::Error, &format!("invalid gamepad-disconnected event: {error}")),
            },
            GamepadButtonDown::TOPIC => match GamepadButtonDown::decode(&payload) {
                Ok(message) => {
                    log(Level::Info, &format!("gamepad {} button {} pressed", message.id, message.button))
                }
                Err(error) => log(Level::Error, &format!("invalid gamepad-button-down event: {error}")),
            },
            GamepadButtonUp::TOPIC => match GamepadButtonUp::decode(&payload) {
                Ok(message) => {
                    log(Level::Info, &format!("gamepad {} button {} released", message.id, message.button))
                }
                Err(error) => log(Level::Error, &format!("invalid gamepad-button-up event: {error}")),
            },
            _ => {}
        }
        None
    }
}

export!(Component);
