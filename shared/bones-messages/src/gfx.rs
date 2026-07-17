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

/// Draws a cached sprite region at a destination position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawSprite {
    /// Identifier supplied by the corresponding `LoadSprite` message.
    pub id: u32,
    /// Destination x coordinate.
    pub dst_x: i32,
    /// Destination y coordinate.
    pub dst_y: i32,
    /// Source-region x coordinate.
    pub src_x: i32,
    /// Source-region y coordinate.
    pub src_y: i32,
    /// Source-region and destination width.
    pub src_w: u32,
    /// Source-region and destination height.
    pub src_h: u32,
}

impl Message for DrawSprite {
    const TOPIC: &'static str = "gfx/draw-sprite";
}

impl EncodeMessage for DrawSprite {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.id)
            .i32(self.dst_x)
            .i32(self.dst_y)
            .i32(self.src_x)
            .i32(self.src_y)
            .u32(self.src_w)
            .u32(self.src_h)
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
            src_x: reader.read_i32()?,
            src_y: reader.read_i32()?,
            src_w: reader.read_u32()?,
            src_h: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

/// Any currently supported `gfx/*` command, decoded by exact topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    Clear(Clear),
    LoadSprite(LoadSprite<'a>),
    DrawSprite(DrawSprite),
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
            src_x: 0,
            src_y: 0,
            src_w: 64,
            src_h: 64,
        };
        assert_eq!(
            Command::decode(DrawSprite::TOPIC, &draw.encode()),
            Ok(Some(Command::DrawSprite(draw)))
        );
    }

    #[test]
    fn fixed_shape_commands_reject_wrong_byte_counts() {
        assert_eq!(
            Command::decode(Clear::TOPIC, &[1, 2, 3]),
            Err(DecodeError::Truncated)
        );
        assert_eq!(
            Command::decode(DrawSprite::TOPIC, &[0; 27]),
            Err(DecodeError::Truncated)
        );
    }
}
