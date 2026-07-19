use crate::{DecodeError, DecodeMessage, Message};

use super::{LoadTilemap, SpawnEntity};

/// Any currently supported `game-core/*` command, decoded by exact topic —
/// the same grouping-enum pattern as `gfx::Command`/`audio::Command`.
// No `Eq`: `SpawnEntity` carries `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    SpawnEntity(SpawnEntity),
    LoadTilemap(LoadTilemap<'a>),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            SpawnEntity::TOPIC => {
                SpawnEntity::decode(payload).map(|value| Some(Self::SpawnEntity(value)))
            }
            LoadTilemap::TOPIC => {
                LoadTilemap::decode(payload).map(|value| Some(Self::LoadTilemap(value)))
            }
            _ => Ok(None),
        }
    }
}
