use crate::{DecodeError, DecodeMessage, Message};

use super::{LoadSound, PlayMusic, PlaySound, SetMusicVolume, StopMusic};

/// Any currently supported `audio/*` command, decoded by exact topic — the
/// same grouping-enum pattern as `gfx::Command`.
// No `Eq`: `PlaySound`/`PlayMusic`/`SetMusicVolume` carry `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    /// Loads a sound from encoded audio bytes (`audio/load-sound`).
    LoadSound(LoadSound<'a>),
    /// Plays a loaded sound once (`audio/play-sound`).
    PlaySound(PlaySound),
    /// Loops a loaded sound as music (`audio/play-music`).
    PlayMusic(PlayMusic),
    /// Stops the music (`audio/stop-music`).
    StopMusic(StopMusic),
    /// Changes the music volume (`audio/set-music-volume`).
    SetMusicVolume(SetMusicVolume),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            LoadSound::TOPIC => {
                LoadSound::decode(payload).map(|value| Some(Self::LoadSound(value)))
            }
            PlaySound::TOPIC => {
                PlaySound::decode(payload).map(|value| Some(Self::PlaySound(value)))
            }
            PlayMusic::TOPIC => {
                PlayMusic::decode(payload).map(|value| Some(Self::PlayMusic(value)))
            }
            StopMusic::TOPIC => {
                StopMusic::decode(payload).map(|value| Some(Self::StopMusic(value)))
            }
            SetMusicVolume::TOPIC => {
                SetMusicVolume::decode(payload).map(|value| Some(Self::SetMusicVolume(value)))
            }
            _ => Ok(None),
        }
    }
}
