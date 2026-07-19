use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Plays a cached sound once, fire-and-forget, at `volume` (linear
/// amplitude: `0.0` silent, `1.0` unity gain — not decibels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaySound {
    pub id: u32,
    pub volume: f32,
}

impl Message for PlaySound {
    const TOPIC: &'static str = "audio/play-sound";
}

impl EncodeMessage for PlaySound {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).f32(self.volume).finish()
    }
}

impl<'a> DecodeMessage<'a> for PlaySound {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            volume: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
