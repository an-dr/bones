use crate::{DecodeError, Reader, Writer};

/// Where a widget tree is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Root {
    /// A floating panel with a title; each owner may show one.
    Window {
        /// The panel's title.
        title: String,
    },
    /// The whole window; its direct children are usually
    /// `Layout::Panel` containers. One owner holds it at a time.
    Screen,
}

const TAG_WINDOW: u8 = 0;
const TAG_SCREEN: u8 = 1;

impl Root {
    pub(super) fn encode_into(&self, writer: Writer) -> Writer {
        match self {
            Root::Window { title } => writer.u8(TAG_WINDOW).str(title),
            Root::Screen => writer.u8(TAG_SCREEN),
        }
    }

    pub(super) fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let tag = reader.read_u8()?;
        match tag {
            TAG_WINDOW => Ok(Root::Window {
                title: reader.read_str()?.to_owned(),
            }),
            TAG_SCREEN => Ok(Root::Screen),
            _ => Err(DecodeError::InvalidTag {
                message: "ui root",
                tag,
            }),
        }
    }
}
