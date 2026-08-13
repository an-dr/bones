/// Horizontal placement of rasterized text relative to its x anchor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// The anchor is the text's left edge.
    #[default]
    Left,
    /// The anchor is the text's horizontal center.
    Center,
    /// The anchor is the text's right edge.
    Right,
}

impl TextAlign {
    pub(crate) const fn encode(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Center => 1,
            Self::Right => 2,
        }
    }

    pub(crate) const fn decode(value: u8) -> Result<Self, crate::DecodeError> {
        match value {
            0 => Ok(Self::Left),
            1 => Ok(Self::Center),
            2 => Ok(Self::Right),
            tag => Err(crate::DecodeError::InvalidTag {
                message: "text alignment",
                tag,
            }),
        }
    }
}
