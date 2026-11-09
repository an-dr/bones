use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension the first time a `tile` container
/// is on screen: the moment to start loading its image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appeared {
    /// The container's id.
    pub id: u32,
}

impl Message for Appeared {
    const TOPIC: &'static str = "ui/appeared";
}

impl EncodeMessage for Appeared {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for Appeared {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
