use crate::DecodeError;

/// What a [`super::Request`] asks for.
///
/// Tags are part of the wire format: add new actions at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Reads the release manifest and compares it with the running version.
    Check,
    /// Downloads and verifies a newer release into its own version folder,
    /// for the launcher to start next time.
    Stage,
    /// Copies the running build into the install location.
    Install,
    /// Reports the version that ran before this one.
    LastApplied,
}

impl Action {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Action::Check => 0,
            Action::Stage => 1,
            Action::Install => 2,
            Action::LastApplied => 3,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> core::result::Result<Self, DecodeError> {
        match tag {
            0 => Ok(Action::Check),
            1 => Ok(Action::Stage),
            2 => Ok(Action::Install),
            3 => Ok(Action::LastApplied),
            tag => Err(DecodeError::InvalidTag {
                message: "upgrader action",
                tag,
            }),
        }
    }
}
