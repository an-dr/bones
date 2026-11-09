use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A labelled on/off box. Kind `"checkbox"`; a click reports `Toggled` with the new state.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkbox {
    /// The caption.
    pub label: String,
    /// Whether it is ticked.
    pub checked: bool,
}

impl WidgetProps for Checkbox {
    const KIND: &'static str = "checkbox";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.label)
            .u8(u8::from(self.checked))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let label = reader.read_str()?.to_owned();
        let checked = reader.read_u8()? != 0;
        reader.finish()?;
        Ok(Self { label, checked })
    }
}
