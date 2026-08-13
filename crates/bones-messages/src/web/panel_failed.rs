use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Reports an asynchronous panel creation or operation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelFailed<'a> {
    pub owner: &'a str,
    pub panel: &'a str,
    pub reason: &'a str,
}

impl Message for PanelFailed<'_> {
    const TOPIC: &'static str = "web/panel-failed";
}

impl EncodeMessage for PanelFailed<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(self.owner)
            .str(self.panel)
            .bytes(self.reason.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for PanelFailed<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            owner: reader.read_str()?,
            panel: reader.read_str()?,
            reason: reader.read_str_rest()?,
        })
    }
}
