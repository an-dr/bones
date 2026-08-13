use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws a circle outline or fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCircle {
    /// Center x coordinate.
    pub x: i32,
    /// Center y coordinate.
    pub y: i32,
    /// Radius in pixels.
    pub radius: u32,
    /// Fills the circle instead of drawing its outline.
    pub filled: bool,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
}

impl Message for DrawCircle {
    const TOPIC: &'static str = "gfx/draw-circle";
}

impl EncodeMessage for DrawCircle {
    fn encode(&self) -> Vec<u8> {
        let (r, g, b, a) = self.color;
        Writer::new()
            .i32(self.x)
            .i32(self.y)
            .u32(self.radius)
            .u8(self.filled as u8)
            .u8(r)
            .u8(g)
            .u8(b)
            .u8(a)
            .u8(self.layer)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawCircle {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_i32()?,
            y: reader.read_i32()?,
            radius: reader.read_u32()?,
            filled: reader.read_u8()? != 0,
            color: (
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ),
            layer: reader.read_u8()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
