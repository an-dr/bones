use super::TextAlign;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Draws a line of text with a font-rendered glyph size in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawText<'a> {
    /// The text to draw. No wrapping — one line.
    pub text: &'a str,
    /// Baseline-independent horizontal anchor, interpreted by `align`.
    pub x: i32,
    /// Baseline-independent top-left y coordinate.
    pub y: i32,
    /// Glyph size in points.
    pub size: u16,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
    /// Skips the camera transform (`gfx/set-camera`) entirely when `true` —
    /// see `DrawRect::screen_space`. `false` is every caller's behavior
    /// from before this field existed.
    pub screen_space: bool,
    /// Horizontal placement relative to `x`.
    pub align: TextAlign,
}

impl Message for DrawText<'_> {
    const TOPIC: &'static str = "gfx/draw-text";
}

impl EncodeMessage for DrawText<'_> {
    fn encode(&self) -> Vec<u8> {
        let (r, g, b, a) = self.color;
        Writer::new()
            .str(self.text)
            .i32(self.x)
            .i32(self.y)
            .u16(self.size)
            .u8(r)
            .u8(g)
            .u8(b)
            .u8(a)
            .u8(self.layer)
            .u8(self.screen_space as u8)
            .u8(self.align.encode())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawText<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let text = reader.read_str()?;
        let x = reader.read_i32()?;
        let y = reader.read_i32()?;
        let size = reader.read_u16()?;
        let color = (
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
        );
        let layer = reader.read_u8()?;
        let screen_space = reader.read_u8()? != 0;
        let align = match reader.read_rest() {
            [] => TextAlign::Left,
            [value] => TextAlign::decode(*value)?,
            _ => return Err(DecodeError::TrailingBytes),
        };
        Ok(Self {
            text,
            x,
            y,
            size,
            color,
            layer,
            screen_space,
            align,
        })
    }
}
