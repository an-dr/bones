use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A push button. Kind `"button"`; a click reports `Clicked` with its id.
/// Build one with `Button::new("Save")`, adding `.disabled(true)` to grey
/// it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// The caption.
    pub label: String,
    /// Drawn greyed out and ignores clicks.
    pub disabled: bool,
}

impl Button {
    /// An enabled button with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    /// Greys the button out when `disabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl WidgetProps for Button {
    const KIND: &'static str = "button";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.label)
            .u8(u8::from(self.disabled))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let button = Self {
            label: reader.read_str()?.to_owned(),
            disabled: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(button)
    }
}
