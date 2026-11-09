use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// Secondary, de-emphasised text. Kind `"weak"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Weak {
    /// The text shown.
    pub text: String,
}

impl WidgetProps for Weak {
    const KIND: &'static str = "weak";

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
