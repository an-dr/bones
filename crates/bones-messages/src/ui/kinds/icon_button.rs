use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A button with a leading icon; `primary` fills it with the accent colour
/// for the screen's main action. Kind `"icon-button"`; a click reports `Clicked`.
#[derive(Debug, Clone, PartialEq)]
pub struct IconButton {
    /// A lucide.dev icon name; empty for none.
    pub icon: String,
    /// The caption.
    pub label: String,
    /// Whether it is the screen's main action.
    pub primary: bool,
}

impl WidgetProps for IconButton {
    const KIND: &'static str = "icon-button";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.icon)
            .str(&self.label)
            .u8(u8::from(self.primary))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let props = Self {
            icon: reader.read_str()?.to_owned(),
            label: reader.read_str()?.to_owned(),
            primary: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(props)
    }
}
