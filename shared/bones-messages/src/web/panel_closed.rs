use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Announces that a panel has closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelClosed<'a> {
    pub owner: &'a str,
    pub panel: &'a str,
}

impl Message for PanelClosed<'_> {
    const TOPIC: &'static str = "web/panel-closed";
}

impl EncodeMessage for PanelClosed<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(self.owner)
            .bytes(self.panel.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for PanelClosed<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            owner: reader.read_str()?,
            panel: reader.read_str_rest()?,
        })
    }
}
