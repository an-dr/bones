use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published back to the owning extension when a `TextEdit` widget's text
/// changes; carries the field's full new content, not a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed<'a> {
    /// The `TextEdit` widget that changed.
    pub id: u32,
    /// The widget's whole new text.
    pub text: &'a str,
}

impl Message for Changed<'_> {
    const TOPIC: &'static str = "ui/changed";
}

impl EncodeMessage for Changed<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.id)
            .bytes(self.text.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Changed<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let id = reader.read_u32()?;
        let text = reader.read_str_rest()?;
        Ok(Self { id, text })
    }
}
