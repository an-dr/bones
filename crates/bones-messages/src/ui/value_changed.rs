use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `slider` widget is dragged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueChanged {
    /// The widget's id.
    pub id: u32,
    /// The new value.
    pub value: f32,
}

impl Message for ValueChanged {
    const TOPIC: &'static str = "ui/value";
}

impl EncodeMessage for ValueChanged {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).f32(self.value).finish()
    }
}

impl<'a> DecodeMessage<'a> for ValueChanged {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            value: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
