use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Confirms that a panel is ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelOpened<'a> {
    pub owner: &'a str,
    pub panel: &'a str,
}

impl Message for PanelOpened<'_> {
    const TOPIC: &'static str = "web/panel-opened";
}

impl EncodeMessage for PanelOpened<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(self.owner)
            .bytes(self.panel.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for PanelOpened<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            owner: reader.read_str()?,
            panel: reader.read_str_rest()?,
        })
    }
}
