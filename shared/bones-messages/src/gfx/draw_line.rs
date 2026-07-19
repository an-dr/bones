use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws a straight line between two points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawLine {
    /// Start x coordinate.
    pub x1: i32,
    /// Start y coordinate.
    pub y1: i32,
    /// End x coordinate.
    pub x2: i32,
    /// End y coordinate.
    pub y2: i32,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
}

impl Message for DrawLine {
    const TOPIC: &'static str = "gfx/draw-line";
}

impl EncodeMessage for DrawLine {
    fn encode(&self) -> Vec<u8> {
        let (r, g, b, a) = self.color;
        Writer::new()
            .i32(self.x1)
            .i32(self.y1)
            .i32(self.x2)
            .i32(self.y2)
            .u8(r)
            .u8(g)
            .u8(b)
            .u8(a)
            .u8(self.layer)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawLine {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x1: reader.read_i32()?,
            y1: reader.read_i32()?,
            x2: reader.read_i32()?,
            y2: reader.read_i32()?,
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
