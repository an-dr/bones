use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// Emphasised text. Kind `"strong"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Strong {
    /// The text shown.
    pub text: String,
}

impl WidgetProps for Strong {
    const KIND: &'static str = "strong";

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
