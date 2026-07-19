use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Scroll wheel input, already normalized for `MouseWheelDirection::Flipped`
/// (natural scrolling) so consumers always see the same sign convention:
/// positive `y` is up/away, positive `x` is right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseWheel {
    pub x: f32,
    pub y: f32,
}

impl Message for MouseWheel {
    const TOPIC: &'static str = "input/mouse-wheel";
}

impl EncodeMessage for MouseWheel {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.x).f32(self.y).finish()
    }
}

impl<'a> DecodeMessage<'a> for MouseWheel {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            x: reader.read_f32()?,
            y: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
