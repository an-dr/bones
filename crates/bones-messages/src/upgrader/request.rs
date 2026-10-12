use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::Action;

/// Asks the `upgrader` module to act on the application it runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// Echoed back on the matching [`super::Result`].
    pub request_id: u32,
    pub action: Action,
}

impl Message for Request {
    const TOPIC: &'static str = "upgrader/request";
}

impl EncodeMessage for Request {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(self.action.tag())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Request {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let action = Action::from_tag(reader.read_u8()?)?;
        reader.finish()?;
        Ok(Self { request_id, action })
    }
}
