use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Stops the current music track, if any. A no-op otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopMusic;

impl Message for StopMusic {
    const TOPIC: &'static str = "audio/stop-music";
}

impl EncodeMessage for StopMusic {
    fn encode(&self) -> Vec<u8> {
        Writer::new().finish()
    }
}

impl<'a> DecodeMessage<'a> for StopMusic {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        Reader::new(payload).finish()?;
        Ok(Self)
    }
}
