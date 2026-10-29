use crate::DecodeError;

/// A transition in an extension's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The extension finished `init` and is running.
    Loaded,
    /// The extension trapped or exceeded a budget and was quarantined.
    Faulted,
    /// The extension's file changed and a replacement is loading.
    Reloading,
    /// The replacement is running.
    Reloaded,
    /// The extension was unloaded and ran `shutdown`.
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
