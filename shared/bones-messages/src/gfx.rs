//! Typed `gfx/*` draw commands shared by extensions and the renderer.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Clears the current draw target to one RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clear {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl Message for Clear {
    const TOPIC: &'static str = "gfx/clear";
}

impl EncodeMessage for Clear {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u8(self.r)
            .u8(self.g)
            .u8(self.b)
            .u8(self.a)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Clear {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            r: reader.read_u8()?,
            g: reader.read_u8()?,
            b: reader.read_u8()?,
            a: reader.read_u8()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

/// Loads a PNG into the renderer's sprite cache without copying its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSprite<'a> {
    /// Application-assigned sprite identifier.
    pub id: u32,
    /// PNG file bytes borrowed from the decoded payload when possible.
    pub png_bytes: &'a [u8],
}

impl Message for LoadSprite<'_> {
    const TOPIC: &'static str = "gfx/load-sprite";
}

impl EncodeMessage for LoadSprite<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).bytes(self.png_bytes).finish()
    }
}

impl<'a> DecodeMessage<'a> for LoadSprite<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let id = reader.read_u32()?;
        Ok(Self {
            id,
            png_bytes: reader.read_rest(),
        })
    }
}

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

/// Sets the world-to-screen camera transform applied to every retained
/// `DrawSprite` at composite time: `screen = (world - (x, y)) * zoom`.
/// Global, not per-sender — one viewport for the whole scene.
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
        };
        reader.finish()?;
        Ok(message)
    }
}

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

/// Draws a line of text with a font-rendered glyph size in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawText<'a> {
    /// The text to draw. No wrapping — one line.
    pub text: &'a str,
    /// Baseline-independent top-left x coordinate.
    pub x: i32,
    /// Baseline-independent top-left y coordinate.
    pub y: i32,
    /// Glyph size in points.
    pub size: u16,
    /// RGBA color.
    pub color: (u8, u8, u8, u8),
    /// Composite order: layers draw bottom-up, ties broken by publish order.
    pub layer: u8,
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
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for DrawText<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            text: reader.read_str()?,
            x: reader.read_i32()?,
            y: reader.read_i32()?,
            size: reader.read_u16()?,
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

/// Any currently supported `gfx/*` command, decoded by exact topic.
// No `Eq`: `SetCamera` carries `f32` fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command<'a> {
    Clear(Clear),
    LoadSprite(LoadSprite<'a>),
    DrawSprite(DrawSprite),
    SetCamera(SetCamera),
    DrawRect(DrawRect),
    DrawLine(DrawLine),
    DrawCircle(DrawCircle),
    DrawText(DrawText<'a>),
}

impl<'a> Command<'a> {
    /// Selects the typed command by topic, returning `Ok(None)` for an
    /// unknown topic and a decode error for a known topic's invalid payload.
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> {
        match topic {
            Clear::TOPIC => Clear::decode(payload).map(|value| Some(Self::Clear(value))),
            LoadSprite::TOPIC => {
                LoadSprite::decode(payload).map(|value| Some(Self::LoadSprite(value)))
            }
            DrawSprite::TOPIC => {
                DrawSprite::decode(payload).map(|value| Some(Self::DrawSprite(value)))
            }
            SetCamera::TOPIC => {
                SetCamera::decode(payload).map(|value| Some(Self::SetCamera(value)))
            }
            DrawRect::TOPIC => DrawRect::decode(payload).map(|value| Some(Self::DrawRect(value))),
            DrawLine::TOPIC => DrawLine::decode(payload).map(|value| Some(Self::DrawLine(value))),
            DrawCircle::TOPIC => {
                DrawCircle::decode(payload).map(|value| Some(Self::DrawCircle(value)))
            }
            DrawText::TOPIC => DrawText::decode(payload).map(|value| Some(Self::DrawText(value))),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_gfx_topics_are_ignored() {
        assert_eq!(Command::decode("input/key-down", b"whatever"), Ok(None));
    }

    #[test]
    fn every_command_round_trips() {
        let clear = Clear {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        assert_eq!(
            Command::decode(Clear::TOPIC, &clear.encode()),
            Ok(Some(Command::Clear(clear)))
        );

        let load = LoadSprite {
            id: 7,
            png_bytes: b"not-really-a-png",
        };
        let encoded = load.encode();
        assert_eq!(
            Command::decode(LoadSprite::TOPIC, &encoded),
            Ok(Some(Command::LoadSprite(load)))
        );

        let draw = DrawSprite {
            id: 7,
            dst_x: 100,
            dst_y: 200,
            dst_w: 96,
            dst_h: 96,
            src_x: 0,
            src_y: 0,
            src_w: 64,
            src_h: 64,
            layer: 3,
            angle: 45.0,
            flip_h: true,
            flip_v: false,
            tint: (255, 128, 64, 255),
        };
        assert_eq!(
            Command::decode(DrawSprite::TOPIC, &draw.encode()),
            Ok(Some(Command::DrawSprite(draw)))
        );

        let camera = SetCamera {
            x: 12.5,
            y: -4.0,
            zoom: 2.0,
        };
        assert_eq!(
            Command::decode(SetCamera::TOPIC, &camera.encode()),
            Ok(Some(Command::SetCamera(camera)))
        );

        let rect = DrawRect {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
            filled: true,
            color: (255, 0, 0, 255),
            layer: 2,
        };
        assert_eq!(
            Command::decode(DrawRect::TOPIC, &rect.encode()),
            Ok(Some(Command::DrawRect(rect)))
        );

        let line = DrawLine {
            x1: 0,
            y1: 0,
            x2: 50,
            y2: 60,
            color: (0, 255, 0, 255),
            layer: 1,
        };
        assert_eq!(
            Command::decode(DrawLine::TOPIC, &line.encode()),
            Ok(Some(Command::DrawLine(line)))
        );

        let circle = DrawCircle {
            x: 5,
            y: 5,
            radius: 15,
            filled: false,
            color: (0, 0, 255, 255),
            layer: 4,
        };
        assert_eq!(
            Command::decode(DrawCircle::TOPIC, &circle.encode()),
            Ok(Some(Command::DrawCircle(circle)))
        );

        let text = DrawText {
            text: "hp: 30/30",
            x: 8,
            y: 8,
            size: 16,
            color: (255, 255, 255, 255),
            layer: 9,
        };
        assert_eq!(
            Command::decode(DrawText::TOPIC, &text.encode()),
            Ok(Some(Command::DrawText(text)))
        );
    }

    #[test]
    fn fixed_shape_commands_reject_wrong_byte_counts() {
        assert_eq!(
            Command::decode(Clear::TOPIC, &[1, 2, 3]),
            Err(DecodeError::Truncated)
        );
        assert_eq!(
            Command::decode(DrawSprite::TOPIC, &[0; 46]),
            Err(DecodeError::Truncated)
        );
    }
}
