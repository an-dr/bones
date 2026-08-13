use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Loads audio bytes (any symphonia-supported format: WAV, MP3, OGG,
/// FLAC, …) into the audio module's cache, keyed by an application-assigned
/// id — the same pattern as `gfx::LoadSprite`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSound<'a> {
    pub id: u32,
    pub bytes: &'a [u8],
}

impl Message for LoadSound<'_> {
    const TOPIC: &'static str = "audio/load-sound";
}

impl EncodeMessage for LoadSound<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).bytes(self.bytes).finish()
    }
}

impl<'a> DecodeMessage<'a> for LoadSound<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let id = reader.read_u32()?;
        Ok(Self {
            id,
            bytes: reader.read_rest(),
        })
    }
}
