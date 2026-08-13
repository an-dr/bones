use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

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
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
