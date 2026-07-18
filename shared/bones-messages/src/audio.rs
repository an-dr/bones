//! Typed `audio/*` commands shared by extensions and the audio module.

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
        Ok(Self { id, bytes: reader.read_rest() })
    }
}

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
        let message = Self { volume: reader.read_f32()? };
        reader.finish()?;
        Ok(message)
    }
}

/// Any currently supported `audio/*` command, decoded by exact topic — the
/// same grouping-enum pattern as `gfx::Command`.
// No `Eq`: `PlaySound`/`PlayMusic`/`SetMusicVolume` carry `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    LoadSound(LoadSound<'a>),
    PlaySound(PlaySound),
    PlayMusic(PlayMusic),
    StopMusic(StopMusic),
    SetMusicVolume(SetMusicVolume),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            LoadSound::TOPIC => LoadSound::decode(payload).map(|value| Some(Self::LoadSound(value))),
            PlaySound::TOPIC => PlaySound::decode(payload).map(|value| Some(Self::PlaySound(value))),
            PlayMusic::TOPIC => PlayMusic::decode(payload).map(|value| Some(Self::PlayMusic(value))),
            StopMusic::TOPIC => StopMusic::decode(payload).map(|value| Some(Self::StopMusic(value))),
            SetMusicVolume::TOPIC => {
                SetMusicVolume::decode(payload).map(|value| Some(Self::SetMusicVolume(value)))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_audio_topics_are_ignored() {
        assert_eq!(Command::decode("input/key-down", b"whatever"), Ok(None));
    }

    #[test]
    fn every_command_round_trips() {
        let load = LoadSound { id: 3, bytes: b"not-really-audio" };
        assert_eq!(
            Command::decode(LoadSound::TOPIC, &load.encode()),
            Ok(Some(Command::LoadSound(load)))
        );

        let play = PlaySound { id: 3, volume: 0.8 };
        assert_eq!(
            Command::decode(PlaySound::TOPIC, &play.encode()),
            Ok(Some(Command::PlaySound(play)))
        );

        let music = PlayMusic { id: 7, volume: 0.5 };
        assert_eq!(
            Command::decode(PlayMusic::TOPIC, &music.encode()),
            Ok(Some(Command::PlayMusic(music)))
        );

        let stop = StopMusic;
        assert_eq!(
            Command::decode(StopMusic::TOPIC, &stop.encode()),
            Ok(Some(Command::StopMusic(stop)))
        );

        let volume = SetMusicVolume { volume: 0.3 };
        assert_eq!(
            Command::decode(SetMusicVolume::TOPIC, &volume.encode()),
            Ok(Some(Command::SetMusicVolume(volume)))
        );
    }

    #[test]
    fn fixed_shape_commands_reject_wrong_byte_counts() {
        assert_eq!(Command::decode(PlaySound::TOPIC, &[0; 7]), Err(DecodeError::Truncated));
        assert_eq!(
            Command::decode(StopMusic::TOPIC, &[0; 1]),
            Err(DecodeError::TrailingBytes),
            "StopMusic reads no fields, so any nonzero payload is trailing, not truncated"
        );
    }
}
