use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Plays a cached sound on loop as the current music track, replacing
/// whatever music was already playing — one active music track at a time,
/// a tactical simplification for this increment, not an architectural
/// limit. `volume` is linear amplitude, same convention as `PlaySound`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayMusic {
    pub id: u32,
    pub volume: f32,
}

impl Message for PlayMusic {
    const TOPIC: &'static str = "audio/play-music";
}

impl EncodeMessage for PlayMusic {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).f32(self.volume).finish()
    }
}

impl<'a> DecodeMessage<'a> for PlayMusic {
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
