//! `renderer/display-changed` (messaging.md): the renderer's own outbound
//! notification, distinct from `gfx/*` (which flows the other way —
//! extension to renderer) — the wire format both the renderer and any
//! extension that wants to react agree on.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published by the renderer immediately after applying a `gfx::SetDisplay`
/// command, carrying the actual resulting window size — fullscreen may not
/// honor the requested `width`/`height` (the OS decides), so this is how a
/// caller learns what really happened instead of assuming its own request
/// took effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayChanged {
    pub width: u32,
    pub height: u32,
}

impl Message for DisplayChanged {
    const TOPIC: &'static str = "renderer/display-changed";
}

impl EncodeMessage for DisplayChanged {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.width).u32(self.height).finish()
    }
}

impl<'a> DecodeMessage<'a> for DisplayChanged {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            width: reader.read_u32()?,
            height: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests;
