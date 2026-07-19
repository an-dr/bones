use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

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
        Writer::new()
            .u32(self.id)
            .str(self.axis)
            .f32(self.value)
            .finish()
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
