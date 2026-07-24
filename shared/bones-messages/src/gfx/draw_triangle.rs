use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws a triangle outline or fill from its three vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawTriangle {
    /// First vertex x coordinate.
    pub x1: i32,
    /// First vertex y coordinate.
    pub y1: i32,
    /// Second vertex x coordinate.
    pub x2: i32,
    /// Second vertex y coordinate.
    pub y2: i32,
    /// Third vertex x coordinate.
    pub x3: i32,
    /// Third vertex y coordinate.
    pub y3: i32,
    /// Fills the triangle instead of drawing its outline.
    pub filled: bool,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
}

impl Message for DrawTriangle {
    const TOPIC: &'static str = "gfx/draw-triangle";
}

impl EncodeMessage for DrawTriangle {
    fn encode(&self) -> Vec<u8> {
        let (r, g, b, a) = self.color;
        Writer::new()
            .i32(self.x1)
            .i32(self.y1)
            .i32(self.x2)
            .i32(self.y2)
            .i32(self.x3)
            .i32(self.y3)
            .u8(self.filled as u8)
            .u8(r)
            .u8(g)
            .u8(b)
            .u8(a)
            .u8(self.layer)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawTriangle {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x1: reader.read_i32()?,
            y1: reader.read_i32()?,
            x2: reader.read_i32()?,
            y2: reader.read_i32()?,
            x3: reader.read_i32()?,
            y3: reader.read_i32()?,
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
