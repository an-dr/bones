//! Keyboard and mouse input messages published by the platform.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

macro_rules! key_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name<'a> {
            /// Platform key name borrowed from the decoded payload when possible.
            pub key: &'a str,
        }

        impl Message for $name<'_> {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name<'_> {
            fn encode(&self) -> Vec<u8> {
                Writer::new().bytes(self.key.as_bytes()).finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name<'a> {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                Ok(Self {
                    key: reader.read_str_rest()?,
                })
            }
        }
    };
}

key_message!(KeyDown, "input/key-down", "A keyboard key was pressed.");
key_message!(KeyUp, "input/key-up", "A keyboard key was released.");

/// Cursor moved; window-relative pixel position plus this event's raw delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMove {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

impl Message for MouseMove {
    const TOPIC: &'static str = "input/mouse-move";
}

impl EncodeMessage for MouseMove {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.x).f32(self.y).f32(self.dx).f32(self.dy).finish()
    }
}

impl<'a> DecodeMessage<'a> for MouseMove {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_f32()?,
            y: reader.read_f32()?,
            dx: reader.read_f32()?,
            dy: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

macro_rules! mouse_button_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[doc = ""]
        #[doc = "`button` is SDL's own code: 1=left, 2=middle, 3=right, 4=x1, 5=x2, 0=unknown."]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            pub button: u8,
            pub x: f32,
            pub y: f32,
        }

        impl Message for $name {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name {
            fn encode(&self) -> Vec<u8> {
                Writer::new().u8(self.button).f32(self.x).f32(self.y).finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                let message = Self {
                    button: reader.read_u8()?,
                    x: reader.read_f32()?,
                    y: reader.read_f32()?,
                };
                reader.finish()?;
                Ok(message)
            }
        }
    };
}

mouse_button_message!(MouseDown, "input/mouse-down", "A mouse button was pressed.");
mouse_button_message!(MouseUp, "input/mouse-up", "A mouse button was released.");

/// Scroll wheel input, already normalized for `MouseWheelDirection::Flipped`
/// (natural scrolling) so consumers always see the same sign convention:
/// positive `y` is up/away, positive `x` is right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseWheel {
    pub x: f32,
    pub y: f32,
}

impl Message for MouseWheel {
    const TOPIC: &'static str = "input/mouse-wheel";
}

impl EncodeMessage for MouseWheel {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.x).f32(self.y).finish()
    }
}

impl<'a> DecodeMessage<'a> for MouseWheel {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_f32()?,
            y: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

/// A gamepad connected. `id` is SDL's joystick instance id — stable for
/// this connection's lifetime, distinguishes multiple gamepads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamepadConnected {
    pub id: u32,
}

impl Message for GamepadConnected {
    const TOPIC: &'static str = "input/gamepad-connected";
}

impl EncodeMessage for GamepadConnected {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for GamepadConnected {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self { id: reader.read_u32()? };
        reader.finish()?;
        Ok(message)
    }
}

/// A previously connected gamepad disconnected; `id` matches the
/// `GamepadConnected` that introduced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamepadDisconnected {
    pub id: u32,
}

impl Message for GamepadDisconnected {
    const TOPIC: &'static str = "input/gamepad-disconnected";
}

impl EncodeMessage for GamepadDisconnected {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for GamepadDisconnected {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self { id: reader.read_u32()? };
        reader.finish()?;
        Ok(message)
    }
}

/// Analog stick/trigger movement, normalized to `[-1.0, 1.0]` (a trigger
/// rests at `0.0` and pulls to `1.0`). `axis` is SDL's own axis name
/// (`"LeftX"`, `"TriggerRight"`, …), the same convention `KeyDown::key` uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamepadAxis<'a> {
    pub id: u32,
    pub axis: &'a str,
    pub value: f32,
}

impl Message for GamepadAxis<'_> {
    const TOPIC: &'static str = "input/gamepad-axis";
}

impl EncodeMessage for GamepadAxis<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).str(self.axis).f32(self.value).finish()
    }
}

impl<'a> DecodeMessage<'a> for GamepadAxis<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            axis: reader.read_str()?,
            value: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

macro_rules! gamepad_button_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[doc = ""]
        #[doc = "`button` is SDL's own button name (\"South\", \"DPadUp\", …)."]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name<'a> {
            pub id: u32,
            pub button: &'a str,
        }

        impl Message for $name<'_> {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name<'_> {
            fn encode(&self) -> Vec<u8> {
                Writer::new().u32(self.id).str(self.button).finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name<'a> {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                let message = Self {
                    id: reader.read_u32()?,
                    button: reader.read_str()?,
                };
                reader.finish()?;
                Ok(message)
            }
        }
    };
}

gamepad_button_message!(GamepadButtonDown, "input/gamepad-button-down", "A gamepad button was pressed.");
gamepad_button_message!(GamepadButtonUp, "input/gamepad-button-up", "A gamepad button was released.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_messages_round_trip_without_changing_the_wire_format() {
        let down = KeyDown { key: "Space" };
        assert_eq!(down.encode(), b"Space");
        assert_eq!(KeyDown::decode(&down.encode()), Ok(down));

        let up = KeyUp { key: "Escape" };
        assert_eq!(KeyUp::decode(&up.encode()), Ok(up));
    }

    #[test]
    fn mouse_messages_round_trip() {
        let mv = MouseMove { x: 12.5, y: -3.0, dx: 1.0, dy: 0.0 };
        assert_eq!(MouseMove::decode(&mv.encode()), Ok(mv));

        let down = MouseDown { button: 1, x: 10.0, y: 20.0 };
        assert_eq!(MouseDown::decode(&down.encode()), Ok(down));

        let up = MouseUp { button: 3, x: 10.0, y: 20.0 };
        assert_eq!(MouseUp::decode(&up.encode()), Ok(up));

        let wheel = MouseWheel { x: 0.0, y: 1.0 };
        assert_eq!(MouseWheel::decode(&wheel.encode()), Ok(wheel));
    }

    #[test]
    fn mouse_messages_reject_wrong_byte_counts() {
        assert_eq!(MouseMove::decode(&[0; 15]), Err(DecodeError::Truncated));
        assert_eq!(MouseDown::decode(&[0; 8]), Err(DecodeError::Truncated));
        assert_eq!(MouseWheel::decode(&[0; 7]), Err(DecodeError::Truncated));
    }

    #[test]
    fn gamepad_messages_round_trip() {
        let connected = GamepadConnected { id: 7 };
        assert_eq!(GamepadConnected::decode(&connected.encode()), Ok(connected));

        let disconnected = GamepadDisconnected { id: 7 };
        assert_eq!(GamepadDisconnected::decode(&disconnected.encode()), Ok(disconnected));

        let axis = GamepadAxis { id: 1, axis: "LeftX", value: -0.5 };
        assert_eq!(GamepadAxis::decode(&axis.encode()), Ok(axis));

        let down = GamepadButtonDown { id: 1, button: "South" };
        assert_eq!(GamepadButtonDown::decode(&down.encode()), Ok(down));

        let up = GamepadButtonUp { id: 1, button: "South" };
        assert_eq!(GamepadButtonUp::decode(&up.encode()), Ok(up));
    }
}
