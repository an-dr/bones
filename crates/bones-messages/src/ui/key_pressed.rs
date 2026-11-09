use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published to the `Screen` owner when a key is pressed that no widget
/// took, such as Escape or an arrow key while nothing has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPressed<'a> {
    /// The key's name, as egui names it: `Escape`, `ArrowLeft`, `Enter`, `A`.
    pub key: &'a str,
}

impl Message for KeyPressed<'_> {
    const TOPIC: &'static str = "ui/key";
}

impl EncodeMessage for KeyPressed<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.key.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for KeyPressed<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            key: reader.read_str_rest()?,
        })
    }
}
