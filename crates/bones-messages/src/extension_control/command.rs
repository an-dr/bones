use crate::{DecodeError, DecodeMessage, Message};

use super::{Load, Reload, Unload};

/// Any runtime extension-control command, selected by exact topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    /// Loads a cataloged extension.
    Load(Load<'a>),
    /// Stops and removes a running extension.
    Unload(Unload<'a>),
    /// Reloads a running extension from its file.
    Reload(Reload<'a>),
}

impl<'a> Command<'a> {
    /// Decodes a known command or returns `Ok(None)` for another topic.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            Load::TOPIC => Load::decode(payload).map(|value| Some(Self::Load(value))),
            Unload::TOPIC => Unload::decode(payload).map(|value| Some(Self::Unload(value))),
            Reload::TOPIC => Reload::decode(payload).map(|value| Some(Self::Reload(value))),
            _ => Ok(None),
        }
    }
}
