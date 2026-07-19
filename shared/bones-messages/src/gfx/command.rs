use crate::{DecodeError, DecodeMessage, Message};

use super::{
    Clear, DrawCircle, DrawLine, DrawRect, DrawSprite, DrawText, LoadSprite, SetCamera,
};

/// Any currently supported `gfx/*` command, decoded by exact topic.
// No `Eq`: `SetCamera` carries `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    Clear(Clear),
    LoadSprite(LoadSprite<'a>),
    DrawSprite(DrawSprite),
    SetCamera(SetCamera),
    DrawRect(DrawRect),
    DrawLine(DrawLine),
    DrawCircle(DrawCircle),
    DrawText(DrawText<'a>),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            Clear::TOPIC => Clear::decode(payload).map(|value| Some(Self::Clear(value))),
            LoadSprite::TOPIC => {
                LoadSprite::decode(payload).map(|value| Some(Self::LoadSprite(value)))
            }
            DrawSprite::TOPIC => {
                DrawSprite::decode(payload).map(|value| Some(Self::DrawSprite(value)))
            }
            SetCamera::TOPIC => {
                SetCamera::decode(payload).map(|value| Some(Self::SetCamera(value)))
            }
            DrawRect::TOPIC => DrawRect::decode(payload).map(|value| Some(Self::DrawRect(value))),
            DrawLine::TOPIC => DrawLine::decode(payload).map(|value| Some(Self::DrawLine(value))),
            DrawCircle::TOPIC => {
                DrawCircle::decode(payload).map(|value| Some(Self::DrawCircle(value)))
            }
            DrawText::TOPIC => DrawText::decode(payload).map(|value| Some(Self::DrawText(value))),
            _ => Ok(None),
        }
    }
}
