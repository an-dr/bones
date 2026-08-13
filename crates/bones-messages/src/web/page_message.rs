use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Carries opaque JSON posted by a panel page to its owning extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMessage<'a> {
    pub owner: &'a str,
    pub panel: &'a str,
    pub json: &'a str,
}

impl Message for PageMessage<'_> {
    const TOPIC: &'static str = "web/page-message";
}

impl EncodeMessage for PageMessage<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(self.owner)
            .str(self.panel)
            .bytes(self.json.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for PageMessage<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            owner: reader.read_str()?,
            panel: reader.read_str()?,
            json: reader.read_str_rest()?,
        })
    }
}
