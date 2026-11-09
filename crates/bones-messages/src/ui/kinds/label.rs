use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// Static text. Kind `"label"`; it reports no events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The text shown.
    pub text: String,
}

impl WidgetProps for Label {
    const KIND: &'static str = "label";

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
