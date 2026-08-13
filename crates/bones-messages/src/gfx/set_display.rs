use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Requests the renderer resize and/or fullscreen-toggle the actual OS
/// window. Doesn't touch the fixed logical resolution every other `gfx/*`
/// coordinate is expressed in (`core/renderer`'s own `Inner::reference_size`,
/// set once from the window's size at startup) — existing content just
/// scales to fit, so nothing else (an extension's world-space draws, a
/// `screen_space` HUD, `game-core`'s camera math) needs to know or care
/// that the window changed size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDisplay {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

impl Message for SetDisplay {
    const TOPIC: &'static str = "gfx/set-display";
}

impl EncodeMessage for SetDisplay {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.width)
            .u32(self.height)
            .u8(self.fullscreen as u8)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for SetDisplay {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            width: reader.read_u32()?,
            height: reader.read_u32()?,
            fullscreen: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(message)
    }
}
