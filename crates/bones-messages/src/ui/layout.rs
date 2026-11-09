use crate::{DecodeError, Reader, Writer};

use super::PanelSide;

/// How a container node arranges its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Top to bottom.
    Vertical,
    /// Left to right.
    Horizontal,
    /// Top to bottom, each child centred horizontally.
    Centred,
    /// Top to bottom inside a scrollable area; the ui module keeps the
    /// scroll position per node id.
    Scroll,
    /// Top to bottom inside a card: the look's faint fill and rounded
    /// corners, full width. A `strong` first child serves as its title.
    Frame,
    /// Row by row, `columns` children per row.
    Grid {
        /// Children per row; zero is drawn as one.
        columns: u16,
    },
    /// One panel of a `Screen`; only meaningful as a direct child of a
    /// `Screen` root.
    Panel {
        /// Which edge the panel occupies.
        side: PanelSide,
    },
}

const TAG_VERTICAL: u8 = 0;
const TAG_HORIZONTAL: u8 = 1;
const TAG_CENTRED: u8 = 2;
const TAG_SCROLL: u8 = 3;
const TAG_FRAME: u8 = 4;
const TAG_GRID: u8 = 5;
const TAG_PANEL: u8 = 6;

impl Layout {
    pub(super) fn encode_into(self, writer: Writer) -> Writer {
        match self {
            Layout::Vertical => writer.u8(TAG_VERTICAL),
            Layout::Horizontal => writer.u8(TAG_HORIZONTAL),
            Layout::Centred => writer.u8(TAG_CENTRED),
            Layout::Scroll => writer.u8(TAG_SCROLL),
            Layout::Frame => writer.u8(TAG_FRAME),
            Layout::Grid { columns } => writer.u8(TAG_GRID).u16(columns),
            Layout::Panel { side } => writer.u8(TAG_PANEL).u8(side.tag()),
        }
    }

    pub(super) fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let tag = reader.read_u8()?;
        Ok(match tag {
            TAG_VERTICAL => Layout::Vertical,
            TAG_HORIZONTAL => Layout::Horizontal,
            TAG_CENTRED => Layout::Centred,
            TAG_SCROLL => Layout::Scroll,
            TAG_FRAME => Layout::Frame,
            TAG_GRID => Layout::Grid {
                columns: reader.read_u16()?,
            },
            TAG_PANEL => Layout::Panel {
                side: PanelSide::from_tag(reader.read_u8()?)?,
            },
            _ => {
                return Err(DecodeError::InvalidTag {
                    message: "ui layout",
                    tag,
                })
            }
        })
    }
}
