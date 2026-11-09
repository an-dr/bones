use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `checkbox` widget is clicked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toggled {
    /// The widget's id.
    pub id: u32,
    /// The new state.
    pub on: bool,
}

impl Message for Toggled {
    const TOPIC: &'static str = "ui/toggled";
}

impl EncodeMessage for Toggled {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).u8(u8::from(self.on)).finish()
    }
}

impl<'a> DecodeMessage<'a> for Toggled {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
            on: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(message)
    }
}
