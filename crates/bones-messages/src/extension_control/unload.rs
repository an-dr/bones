use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Requests orderly shutdown and release of a running extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unload<'a> {
    pub extension: &'a str,
}

impl Message for Unload<'_> {
    const TOPIC: &'static str = "core/extensions/unload";
}

impl EncodeMessage for Unload<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.extension.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for Unload<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            extension: reader.read_str_rest()?,
        })
    }
}
