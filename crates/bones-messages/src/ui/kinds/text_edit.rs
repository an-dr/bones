use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A single-line text field. Kind `"text-edit"`; an edit reports `Changed`
/// with its id and the new text, and Enter reports `Submitted`. Build one
/// with `TextEdit::new(text)`, then `.hint(...)`, `.disabled(...)`, or
/// `.focused(true)` as needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The field's current text.
    pub text: String,
    /// Shown greyed while the field is empty; empty for none.
    pub hint: String,
    /// Drawn greyed out and cannot be edited.
    pub disabled: bool,
    /// Takes the keyboard focus, as a dialog's first field does.
    pub focused: bool,
}

impl TextEdit {
    /// An enabled, unfocused field holding `text`, with no hint.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hint: String::new(),
            disabled: false,
            focused: false,
        }
    }

    /// Shows `hint` while the field is empty.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// Greys the field out when `disabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Gives the field the keyboard focus when `focused`.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl WidgetProps for TextEdit {
    const KIND: &'static str = "text-edit";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.text)
            .str(&self.hint)
            .u8(u8::from(self.disabled))
            .u8(u8::from(self.focused))
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let edit = Self {
            text: reader.read_str()?.to_owned(),
            hint: reader.read_str()?.to_owned(),
            disabled: reader.read_u8()? != 0,
            focused: reader.read_u8()? != 0,
        };
        reader.finish()?;
        Ok(edit)
    }
}
