use crate::DecodeError;

/// A transition in an extension's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Loaded,
    Faulted,
    Reloading,
    Reloaded,
    Stopped,
}

impl Event {
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Loaded => 0,
            Self::Faulted => 1,
            Self::Reloading => 2,
            Self::Reloaded => 3,
            Self::Stopped => 4,
        }
    }

    pub(super) fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Self::Loaded),
            1 => Ok(Self::Faulted),
            2 => Ok(Self::Reloading),
            3 => Ok(Self::Reloaded),
            4 => Ok(Self::Stopped),
            tag => Err(DecodeError::InvalidTag {
                message: "lifecycle event",
                tag,
            }),
        }
    }
}
