use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A push button. Kind `"button"`; a click reports `Clicked` with its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// The caption.
    pub label: String,
}

impl WidgetProps for Button {
    const KIND: &'static str = "button";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.label).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let label = reader.read_str()?.to_owned();
        reader.finish()?;
        Ok(Self { label })
    }
}
