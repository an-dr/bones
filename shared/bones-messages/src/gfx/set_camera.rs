use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Sets the world-to-screen camera transform applied to every retained
/// `DrawSprite` at composite time: `screen = (world - (x, y)) * zoom`.
/// Global, not per-sender — one viewport for the whole scene.
// No `Eq`: all fields are `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetCamera {
    /// World x coordinate the viewport is centered/anchored on.
    pub x: f32,
    /// World y coordinate the viewport is centered/anchored on.
    pub y: f32,
    /// Scale factor; 1.0 is unscaled.
    pub zoom: f32,
}

impl Message for SetCamera {
    const TOPIC: &'static str = "gfx/set-camera";
}

impl EncodeMessage for SetCamera {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.x).f32(self.y).f32(self.zoom).finish()
    }
}

impl<'a> DecodeMessage<'a> for SetCamera {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_f32()?,
            y: reader.read_f32()?,
            zoom: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
