use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Adjusts the current music track's volume in real time (linear
/// amplitude). Has no effect on already-fired `PlaySound` effects — those
/// are fire-and-forget with no retained handle to adjust.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetMusicVolume {
    pub volume: f32,
}

impl Message for SetMusicVolume {
    const TOPIC: &'static str = "audio/set-music-volume";
}

impl EncodeMessage for SetMusicVolume {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.volume).finish()
    }
}

impl<'a> DecodeMessage<'a> for SetMusicVolume {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            volume: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
