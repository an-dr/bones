use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Fixed logical coordinate space used by renderer draw commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalCanvas {
    pub width: u32,
    pub height: u32,
}

impl Message for LogicalCanvas {
    const TOPIC: &'static str = "renderer/logical-canvas";
}

impl EncodeMessage for LogicalCanvas {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.width).u32(self.height).finish()
    }
}

impl<'a> DecodeMessage<'a> for LogicalCanvas {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            width: reader.read_u32()?,
            height: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
