use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A section title. Kind `"heading"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    /// The title.
    pub text: String,
}

impl WidgetProps for Heading {
    const KIND: &'static str = "heading";

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
