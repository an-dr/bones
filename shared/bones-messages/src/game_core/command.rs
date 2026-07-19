use crate::{DecodeError, DecodeMessage, Message};

use super::SpawnEntity;

/// Any currently supported `game-core/*` command, decoded by exact topic —
/// the same grouping-enum pattern as `gfx::Command`/`audio::Command`.
// No `Eq`: `SpawnEntity` carries `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    SpawnEntity(SpawnEntity),
}

impl Command {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &[u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            SpawnEntity::TOPIC => {
                SpawnEntity::decode(payload).map(|value| Some(Self::SpawnEntity(value)))
            }
            _ => Ok(None),
        }
    }
}
