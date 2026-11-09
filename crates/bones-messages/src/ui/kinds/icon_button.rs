use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A button with a leading icon; `primary` fills it with the accent colour
/// for the screen's main action. Kind `"icon-button"`; a click reports
/// `Clicked`. Build one with `IconButton::new("download", "Save")`, then
/// `.primary(true)` or `.disabled(true)`.
#[derive(Debug, Clone, PartialEq)]
pub struct IconButton {
    /// A lucide.dev icon name; empty for none.
    pub icon: String,
    /// The caption.
    pub label: String,
    /// Whether it is the screen's main action.
    pub primary: bool,
    /// Drawn greyed out and ignores clicks.
    pub disabled: bool,
}

impl IconButton {
    /// An enabled, ordinary button with `icon` (a lucide.dev name) and `label`.
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            primary: false,
            disabled: false,
        }
    }

    /// Marks it as the screen's main action.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// Greys it out when `disabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl WidgetProps for IconButton {
    const KIND: &'static str = "icon-button";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.icon)
            .str(&self.label)
            .u8(u8::from(self.primary))
            .u8(u8::from(self.disabled))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let button = Self {
            icon: reader.read_str()?.to_owned(),
            label: reader.read_str()?.to_owned(),
            primary: reader.read_u8()? != 0,
            disabled: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(button)
    }
}
