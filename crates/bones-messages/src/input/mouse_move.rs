use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Cursor moved; window-relative pixel position plus this event's raw delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMove {
    /// Cursor x in window coordinates.
    pub x: f32,
    /// Cursor y in window coordinates.
    pub y: f32,
    /// Movement along x since the previous event.
    pub dx: f32,
    /// Movement along y since the previous event.
    pub dy: f32,
}

impl Message for MouseMove {
    const TOPIC: &'static str = "input/mouse-move";
}

impl EncodeMessage for MouseMove {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .f32(self.x)
            .f32(self.y)
            .f32(self.dx)
            .f32(self.dy)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for MouseMove {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_f32()?,
            y: reader.read_f32()?,
            dx: reader.read_f32()?,
            dy: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
