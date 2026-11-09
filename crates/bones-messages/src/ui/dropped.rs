use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `draggable` container is
/// dropped on a `drop-target` one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dropped {
    /// The `drop-target` container's id.
    pub id: u32,
    /// The dropped `draggable` container's id.
    pub payload: u32,
}

impl Message for Dropped {
    const TOPIC: &'static str = "ui/dropped";
}

impl EncodeMessage for Dropped {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).u32(self.payload).finish()
    }
}

impl<'a> DecodeMessage<'a> for Dropped {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            payload: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
