use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws an axis-aligned rectangle outline or fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawRect {
    /// Top-left x coordinate.
    pub x: i32,
    /// Top-left y coordinate.
    pub y: i32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
    /// Fills the rectangle instead of drawing its outline.
    pub filled: bool,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
    /// Skips the camera transform (`gfx/set-camera`) entirely when `true` —
    /// `x`/`y`/`w`/`h` are literal screen pixels regardless of camera
    /// position or zoom, for HUD/menu content that must stay put as the
    /// camera pans. `false` is every caller's behavior from before this
    /// field existed.
    pub screen_space: bool,
}

impl Message for DrawRect {
    const TOPIC: &'static str = "gfx/draw-rect";
}

impl EncodeMessage for DrawRect {
    fn encode(&self) -> Vec<u8> {
        let (r, g, b, a) = self.color;
        Writer::new()
            .i32(self.x)
            .i32(self.y)
            .u32(self.w)
            .u32(self.h)
            .u8(self.filled as u8)
            .u8(r)
            .u8(g)
            .u8(b)
            .u8(a)
            .u8(self.layer)
            .u8(self.screen_space as u8)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawRect {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_i32()?,
            y: reader.read_i32()?,
            w: reader.read_u32()?,
            h: reader.read_u32()?,
            filled: reader.read_u8()? != 0,
            color: (
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ),
            layer: reader.read_u8()?,
            screen_space: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(message)
    }
}
