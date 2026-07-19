//! Keyboard and mouse input messages published by the platform.

mod gamepad_axis;
mod gamepad_button;
mod gamepad_connected;
mod gamepad_disconnected;
mod key;
mod mouse_button;
mod mouse_move;
mod mouse_wheel;

pub use gamepad_axis::GamepadAxis;
pub use gamepad_button::{GamepadButtonDown, GamepadButtonUp};
pub use gamepad_connected::GamepadConnected;
pub use gamepad_disconnected::GamepadDisconnected;
pub use key::{KeyDown, KeyUp};
pub use mouse_button::{MouseDown, MouseUp};
pub use mouse_move::MouseMove;
pub use mouse_wheel::MouseWheel;

#[cfg(test)]
mod tests;
