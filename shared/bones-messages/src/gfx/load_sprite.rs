use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Loads a PNG into the renderer's sprite cache without copying its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSprite<'a> {
    /// Application-assigned sprite identifier.
    pub id: u32,
    /// PNG file bytes borrowed from the decoded payload when possible.
    pub png_bytes: &'a [u8],
}

impl Message for LoadSprite<'_> {
    const TOPIC: &'static str = "gfx/load-sprite";
}

impl EncodeMessage for LoadSprite<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).bytes(self.png_bytes).finish()
    }
}

impl<'a> DecodeMessage<'a> for LoadSprite<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let id = reader.read_u32()?;
        Ok(Self {
            id,
            png_bytes: reader.read_rest(),
        })
    }
}
