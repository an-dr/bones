use crate::DecodeError;

/// What a [`super::Request`] asks for.
///
/// Tags are part of the wire format: add new actions at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Writes every part of the registration; repairs a partial one.
    Register,
    /// Removes every part of the registration; the application's files stay.
    Unregister,
    /// Reports whether every part of the registration is present.
    Status,
}

impl Action {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Action::Register => 0,
            Action::Unregister => 1,
            Action::Status => 2,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> core::result::Result<Self, DecodeError> {
        match tag {
            0 => Ok(Action::Register),
            1 => Ok(Action::Unregister),
            2 => Ok(Action::Status),
            tag => Err(DecodeError::InvalidTag {
                message: "desktop action",
                tag,
            }),
        }
    }
}
