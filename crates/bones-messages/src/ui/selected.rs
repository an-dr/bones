use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when an option of a `select` widget is chosen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selected {
    /// The widget's id.
    pub id: u32,
    /// The chosen option's index.
    pub index: u32,
}

impl Message for Selected {
    const TOPIC: &'static str = "ui/selected";
}

impl EncodeMessage for Selected {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).u32(self.index).finish()
    }
}

impl<'a> DecodeMessage<'a> for Selected {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            index: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
