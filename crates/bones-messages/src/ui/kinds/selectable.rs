use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A label that shows whether it is the selected one, as in a segmented choice. Kind `"selectable"`; a click reports `Clicked`.
#[derive(Debug, Clone, PartialEq)]
pub struct Selectable {
    /// The caption.
    pub label: String,
    /// Whether it is drawn selected.
    pub selected: bool,
}

impl WidgetProps for Selectable {
    const KIND: &'static str = "selectable";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.label)
            .u8(u8::from(self.selected))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let label = reader.read_str()?.to_owned();
        let selected = reader.read_u8()? != 0;
        reader.finish()?;
        Ok(Self { label, selected })
    }
}
