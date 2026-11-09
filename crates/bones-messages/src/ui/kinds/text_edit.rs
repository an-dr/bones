use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A single-line text field. Kind `"text-edit"`; an edit reports
/// `Changed` with its id and the new text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The field's current text.
    pub text: String,
}

impl WidgetProps for TextEdit {
    const KIND: &'static str = "text-edit";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.text).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let text = reader.read_str()?.to_owned();
        reader.finish()?;
        Ok(Self { text })
    }
}
