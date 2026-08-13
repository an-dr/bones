use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Announces that the application is beginning its orderly close sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseRequested;

impl Message for CloseRequested {
    const TOPIC: &'static str = "window/close-requested";
}

impl EncodeMessage for CloseRequested {
    fn encode(&self) -> Vec<u8> {
        Writer::new().finish()
    }
}

impl<'a> DecodeMessage<'a> for CloseRequested {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        Reader::new(payload).finish()?;
        Ok(Self)
    }
}
