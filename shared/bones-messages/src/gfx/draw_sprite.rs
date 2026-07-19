use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws a cached sprite region at a destination rectangle, on a given
/// composite layer (`gfx/*` batches are composited layers bottom-up;
/// `docs/design/presentation.md`), with optional rotation, flip, and tint.
// No `Eq`: `angle` is `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawSprite {
    /// Identifier supplied by the corresponding `LoadSprite` message.
    pub id: u32,
    /// Destination x coordinate.
    pub dst_x: i32,
    /// Destination y coordinate.
    pub dst_y: i32,
    /// Destination width — independent of `src_w`, i.e. this is the scale.
    pub dst_w: u32,
    /// Destination height — independent of `src_h`, i.e. this is the scale.
    pub dst_h: u32,
    /// Source-region x coordinate.
    pub src_x: i32,
    /// Source-region y coordinate.
    pub src_y: i32,
    /// Source-region width.
    pub src_w: u32,
    /// Source-region height.
    pub src_h: u32,
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
    /// Rotation in degrees, clockwise, about the destination rect's center.
    pub angle: f32,
    /// Mirror horizontally before rotating.
    pub flip_h: bool,
    /// Mirror vertically before rotating.
    pub flip_v: bool,
    /// Color modulation; `(255, 255, 255, 255)` draws the texture unmodified.
    pub tint: (u8, u8, u8, u8),
}

impl Message for DrawSprite {
    const TOPIC: &'static str = "gfx/draw-sprite";
}

impl EncodeMessage for DrawSprite {
    fn encode(&self) -> Vec<u8> {
        let (tint_r, tint_g, tint_b, tint_a) = self.tint;
        Writer::new()
            .u32(self.id)
            .i32(self.dst_x)
            .i32(self.dst_y)
            .u32(self.dst_w)
            .u32(self.dst_h)
            .i32(self.src_x)
            .i32(self.src_y)
            .u32(self.src_w)
            .u32(self.src_h)
            .u8(self.layer)
            .f32(self.angle)
            .u8(self.flip_h as u8)
            .u8(self.flip_v as u8)
            .u8(tint_r)
            .u8(tint_g)
            .u8(tint_b)
            .u8(tint_a)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawSprite {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            dst_x: reader.read_i32()?,
            dst_y: reader.read_i32()?,
            dst_w: reader.read_u32()?,
            dst_h: reader.read_u32()?,
            src_x: reader.read_i32()?,
            src_y: reader.read_i32()?,
            src_w: reader.read_u32()?,
            src_h: reader.read_u32()?,
            layer: reader.read_u8()?,
            angle: reader.read_f32()?,
            flip_h: reader.read_u8()? != 0,
            flip_v: reader.read_u8()? != 0,
            tint: (
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ),
        };
        reader.finish()?;
        Ok(message)
    }
}
