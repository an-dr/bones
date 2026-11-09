use crate::DecodeError;

/// Where a `Layout::Panel` sits on a `Screen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelSide {
    /// Full width, along the top edge.
    Top,
    /// Full width, along the bottom edge.
    Bottom,
    /// Between top and bottom, along the left edge.
    Left,
    /// Between top and bottom, along the right edge.
    Right,
    /// What remains after the other panels.
    Central,
}

impl PanelSide {
    pub(super) fn tag(self) -> u8 {
        match self {
            PanelSide::Top => 0,
            PanelSide::Bottom => 1,
            PanelSide::Left => 2,
            PanelSide::Right => 3,
            PanelSide::Central => 4,
        }
    }

    pub(super) fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        Ok(match tag {
            0 => PanelSide::Top,
            1 => PanelSide::Bottom,
            2 => PanelSide::Left,
            3 => PanelSide::Right,
            4 => PanelSide::Central,
            _ => {
                return Err(DecodeError::InvalidTag {
                    message: "ui panel side",
                    tag,
                })
            }
        })
    }
}
