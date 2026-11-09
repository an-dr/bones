use crate::{DecodeError, Reader, Writer};

use super::PanelSide;

/// How a container node arranges its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Its children can be dragged as one, carrying this container's id;
    /// see `DropTarget`.
    Draggable,
    /// Highlights while a `Draggable` hovers it; a drop reports `Dropped`
    /// with this container's id and the dragged one's.
    DropTarget,
    /// The first child is drawn; the others form the menu a right click
    /// on it opens. Menu items report their own events.
    ContextMenu,
    /// The first child is drawn; the others appear while the pointer
    /// rests on it.
    Tooltip,
    /// Top to bottom; whenever anything inside changes, the new content
    /// fades in and rises into place. Not on first appearance.
    Fade,
    /// A media tile: an uploaded image cover-cropped into the box, pulsing
    /// until it arrives. The children appear in a bar along the bottom
    /// while the pointer is over it. A click elsewhere on it reports
    /// `Clicked` with this container's id.
    Tile {
        /// The owner's image handle.
        image: u32,
        /// Box width in points.
        width: u16,
        /// Box height in points.
        height: u16,
        /// A Unicode code point drawn in the top-right corner while the
        /// pointer is elsewhere, such as a `ui::icons` glyph; `0` for none.
        badge: u32,
    },
    /// A full-window preview over everything: the image fitted on a dark
    /// backdrop, a spinner until it arrives, and the children in a bar
    /// along the bottom. Its close button reports `Clicked`, its previous
    /// and next buttons `Selected` with index 0 and 1, all with this
    /// container's id.
    Stage {
        /// The owner's image handle.
        image: u32,
        /// Whether to offer a previous button.
        previous: bool,
        /// Whether to offer a next button.
        next: bool,
    },
}

const TAG_VERTICAL: u8 = 0;
const TAG_HORIZONTAL: u8 = 1;
const TAG_CENTRED: u8 = 2;
const TAG_SCROLL: u8 = 3;
const TAG_FRAME: u8 = 4;
const TAG_GRID: u8 = 5;
const TAG_PANEL: u8 = 6;
const TAG_DRAGGABLE: u8 = 7;
const TAG_DROP_TARGET: u8 = 8;
const TAG_CONTEXT_MENU: u8 = 9;
const TAG_TOOLTIP: u8 = 10;
const TAG_FADE: u8 = 11;
const TAG_TILE: u8 = 12;
const TAG_STAGE: u8 = 13;

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
            Layout::Draggable => writer.u8(TAG_DRAGGABLE),
            Layout::DropTarget => writer.u8(TAG_DROP_TARGET),
            Layout::ContextMenu => writer.u8(TAG_CONTEXT_MENU),
            Layout::Tooltip => writer.u8(TAG_TOOLTIP),
            Layout::Fade => writer.u8(TAG_FADE),
            Layout::Tile {
                image,
                width,
                height,
                badge,
            } => writer
                .u8(TAG_TILE)
                .u32(image)
                .u16(width)
                .u16(height)
                .u32(badge),
            Layout::Stage {
                image,
                previous,
                next,
            } => writer
                .u8(TAG_STAGE)
                .u32(image)
                .u8(u8::from(previous))
                .u8(u8::from(next)),
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
            TAG_DRAGGABLE => Layout::Draggable,
            TAG_DROP_TARGET => Layout::DropTarget,
            TAG_CONTEXT_MENU => Layout::ContextMenu,
            TAG_TOOLTIP => Layout::Tooltip,
            TAG_FADE => Layout::Fade,
            TAG_TILE => Layout::Tile {
                image: reader.read_u32()?,
                width: reader.read_u16()?,
                height: reader.read_u16()?,
                badge: reader.read_u32()?,
            },
            TAG_STAGE => Layout::Stage {
                image: reader.read_u32()?,
                previous: reader.read_u8()? != 0,
                next: reader.read_u8()? != 0,
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
