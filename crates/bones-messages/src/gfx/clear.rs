use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Clears the current draw target to one RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clear {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl Message for Clear {
    const TOPIC: &'static str = "gfx/clear";
}

impl EncodeMessage for Clear {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u8(self.r)
            .u8(self.g)
            .u8(self.b)
            .u8(self.a)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Clear {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            r: reader.read_u8()?,
            g: reader.read_u8()?,
            b: reader.read_u8()?,
            a: reader.read_u8()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
