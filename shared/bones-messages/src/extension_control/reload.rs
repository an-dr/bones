use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Requests replacement of a running extension from its catalog path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reload<'a> {
    pub extension: &'a str,
}

impl Message for Reload<'_> {
    const TOPIC: &'static str = "core/extensions/reload";
}

impl EncodeMessage for Reload<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.extension.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for Reload<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            extension: reader.read_str_rest()?,
        })
    }
}
