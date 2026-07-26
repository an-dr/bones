use crate::{DecodeError, DecodeMessage, Message};

use super::{
    Clear, ClearDrawBatch, DrawCircle, DrawLine, DrawRect, DrawSprite, DrawText, DrawTriangle,
    LoadSprite, SetCamera, SetDisplay,
};

/// Any currently supported `gfx/*` command, decoded by exact topic.
// No `Eq`: `SetCamera` carries `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    Clear(Clear),
    ClearDrawBatch(ClearDrawBatch),
    LoadSprite(LoadSprite<'a>),
    DrawSprite(DrawSprite),
    SetCamera(SetCamera),
    SetDisplay(SetDisplay),
    DrawRect(DrawRect),
    DrawLine(DrawLine),
    DrawCircle(DrawCircle),
    DrawText(DrawText<'a>),
    DrawTriangle(DrawTriangle),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            Clear::TOPIC => Clear::decode(payload).map(|value| Some(Self::Clear(value))),
            ClearDrawBatch::TOPIC => {
                ClearDrawBatch::decode(payload).map(|value| Some(Self::ClearDrawBatch(value)))
            }
            LoadSprite::TOPIC => {
                LoadSprite::decode(payload).map(|value| Some(Self::LoadSprite(value)))
            }
            DrawSprite::TOPIC => {
                DrawSprite::decode(payload).map(|value| Some(Self::DrawSprite(value)))
            }
            SetCamera::TOPIC => {
                SetCamera::decode(payload).map(|value| Some(Self::SetCamera(value)))
            }
            SetDisplay::TOPIC => {
                SetDisplay::decode(payload).map(|value| Some(Self::SetDisplay(value)))
            }
            DrawRect::TOPIC => DrawRect::decode(payload).map(|value| Some(Self::DrawRect(value))),
            DrawLine::TOPIC => DrawLine::decode(payload).map(|value| Some(Self::DrawLine(value))),
            DrawCircle::TOPIC => {
                DrawCircle::decode(payload).map(|value| Some(Self::DrawCircle(value)))
            }
            DrawText::TOPIC => DrawText::decode(payload).map(|value| Some(Self::DrawText(value))),
            DrawTriangle::TOPIC => {
                DrawTriangle::decode(payload).map(|value| Some(Self::DrawTriangle(value)))
            }
            _ => Ok(None),
        }
    }
}
