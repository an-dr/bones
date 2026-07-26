use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Replaces the publishing sender's retained draw batch with an empty batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearDrawBatch;

impl Message for ClearDrawBatch {
    const TOPIC: &'static str = "gfx/clear-draw-batch";
}

impl EncodeMessage for ClearDrawBatch {
    fn encode(&self) -> Vec<u8> {
        Writer::new().finish()
    }
}

impl<'a> DecodeMessage<'a> for ClearDrawBatch {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        Reader::new(payload).finish()?;
        Ok(Self)
    }
}
