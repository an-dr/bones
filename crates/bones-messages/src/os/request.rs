use super::Action;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Asks the `os` module to perform one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request<'a> {
    /// Echoed on [`super::Reply`] with the host-stamped requester.
    /// Unique among this caller's outstanding requests.
    pub request_id: u32,
    /// What to do with `value`.
    pub action: Action,
    /// The action's one argument: a URL, a path, a dialog title, or the text
    /// to copy. Empty for an action that takes none.
    pub value: &'a str,
}

impl Message for Request<'_> {
    const TOPIC: &'static str = "os/request";
}

impl EncodeMessage for Request<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(self.action.tag())
            .str(self.value)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Request<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let action = Action::from_tag(reader.read_u8()?)?;
        let value = reader.read_str()?;
        reader.finish()?;
        Ok(Self {
            request_id,
            action,
            value,
        })
    }
}
