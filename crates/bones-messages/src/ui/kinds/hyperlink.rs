use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A link the ui module opens in the system browser. Kind `"hyperlink"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Hyperlink {
    /// The text shown.
    pub label: String,
    /// The address opened.
    pub url: String,
}

impl WidgetProps for Hyperlink {
    const KIND: &'static str = "hyperlink";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.label).str(&self.url).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let label = reader.read_str()?.to_owned();
        let url = reader.read_str()?.to_owned();
        reader.finish()?;
        Ok(Self { label, url })
    }
}
