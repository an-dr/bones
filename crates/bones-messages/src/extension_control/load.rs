use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Requests activation of a discovered extension by its unique name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Load<'a> {
    pub extension: &'a str,
}

impl Message for Load<'_> {
    const TOPIC: &'static str = "core/extensions/load";
}

impl EncodeMessage for Load<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.extension.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for Load<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            extension: reader.read_str_rest()?,
        })
    }
}
