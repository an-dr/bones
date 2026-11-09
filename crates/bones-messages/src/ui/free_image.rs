use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Forgets an image the sender uploaded with `SetImage`; widgets still
/// naming its handle draw an empty box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeImage {
    /// The handle it was uploaded under.
    pub handle: u32,
}

impl Message for FreeImage {
    const TOPIC: &'static str = "ui/image-free";
}

impl EncodeMessage for FreeImage {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.handle).finish()
    }
}

impl<'a> DecodeMessage<'a> for FreeImage {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            handle: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
