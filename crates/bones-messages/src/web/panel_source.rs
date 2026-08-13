use crate::DecodeError;

/// Initial content for a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelSource<'a> {
    Html(&'a str),
    Url(&'a str),
}

impl PanelSource<'_> {
    pub(crate) fn tag(&self) -> u8 {
        match self {
            Self::Html(_) => 0,
            Self::Url(_) => 1,
        }
    }

    pub(crate) fn value(&self) -> &str {
        match self {
            Self::Html(value) | Self::Url(value) => value,
        }
    }

    pub(crate) fn from_tag(tag: u8, value: &str) -> Result<PanelSource<'_>, DecodeError> {
        match tag {
            0 => Ok(PanelSource::Html(value)),
            1 => Ok(PanelSource::Url(value)),
            tag => Err(DecodeError::InvalidTag {
                message: "web panel source",
                tag,
            }),
        }
    }
}
