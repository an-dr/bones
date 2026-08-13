use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

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
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
