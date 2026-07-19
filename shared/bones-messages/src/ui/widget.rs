use crate::{DecodeError, Reader, Writer};

/// One widget in a `Spec`. `TextEdit` and `Button` carry an
/// extension-assigned `id` so `Clicked`/`Changed` can name the widget that
/// produced them; `Label` has no interaction, so no id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Widget<'a> {
    Label { text: &'a str },
    TextEdit { id: u32, text: &'a str },
    Button { id: u32, label: &'a str },
}

const TAG_LABEL: u8 = 0;
const TAG_TEXT_EDIT: u8 = 1;
const TAG_BUTTON: u8 = 2;

impl<'a> Widget<'a> {
    pub(super) fn encode_into(&self, writer: Writer) -> Writer {
        match self {
            Widget::Label { text } => writer.u8(TAG_LABEL).str(text),
            Widget::TextEdit { id, text } => writer.u8(TAG_TEXT_EDIT).u32(*id).str(text),
            Widget::Button { id, label } => writer.u8(TAG_BUTTON).u32(*id).str(label),
        }
    }

    pub(super) fn decode(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let tag = reader.read_u8()?;
        match tag {
            TAG_LABEL => Ok(Widget::Label {
                text: reader.read_str()?,
            }),
            TAG_TEXT_EDIT => Ok(Widget::TextEdit {
                id: reader.read_u32()?,
                text: reader.read_str()?,
            }),
            TAG_BUTTON => Ok(Widget::Button {
                id: reader.read_u32()?,
                label: reader.read_str()?,
            }),
            _ => Err(DecodeError::InvalidTag {
                message: "ui widget",
                tag,
            }),
        }
    }
}
