use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `Button` widget is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clicked {
    pub id: u32,
}

impl Message for Clicked {
    const TOPIC: &'static str = "ui/clicked";
}

impl EncodeMessage for Clicked {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for Clicked {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
