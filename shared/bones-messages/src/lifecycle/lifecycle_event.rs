use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::Event;

/// A lifecycle transition and the extension it concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleEvent<'a> {
    /// The transition that occurred.
    pub event: Event,
    /// Extension name borrowed from the decoded payload when possible.
    pub extension: &'a str,
}

impl Message for LifecycleEvent<'_> {
    const TOPIC: &'static str = "core/lifecycle";
}

impl EncodeMessage for LifecycleEvent<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u8(self.event.tag())
            .bytes(self.extension.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for LifecycleEvent<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let event = Event::from_tag(reader.read_u8()?)?;
        let extension = reader.read_str_rest()?;
        Ok(Self { event, extension })
    }
}
