use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when Enter is pressed in a
/// `text-edit` widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submitted {
    /// The `text-edit` widget's id.
    pub id: u32,
}

impl Message for Submitted {
    const TOPIC: &'static str = "ui/submitted";
}

impl EncodeMessage for Submitted {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for Submitted {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
