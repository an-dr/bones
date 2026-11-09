use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `scroll` container with a
/// non-zero id is scrolled near its end; once each time its content grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReachedEnd {
    /// The container's id.
    pub id: u32,
}

impl Message for ReachedEnd {
    const TOPIC: &'static str = "ui/reached-end";
}

impl EncodeMessage for ReachedEnd {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for ReachedEnd {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
