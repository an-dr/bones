use crate::DecodeError;

/// What the caller wants done.
///
/// Tags are part of the wire format: add new actions at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Reads the system clipboard's text. `value` is ignored.
    ReadClipboard,
    /// Replaces the system clipboard's text with `value`.
    WriteClipboard,
    /// Opens `value` in the user's browser. A host is expected to refuse a
    /// scheme it does not trust rather than hand an arbitrary string to the
    /// shell.
    OpenUrl,
    /// Asks the user to choose one file, with `value` as the dialog title.
    PickFile,
    /// Asks the user to choose one folder, with `value` as the dialog title.
    PickFolder,
    /// Reveals the directory `value` in the desktop's file manager.
    OpenDirectory,
    /// Fetches `value` over HTTPS, answering with
    /// `"{content-type};base64,{data}"` — a self-describing shape the caller
    /// can turn into a data URI or decode directly. Intended for small payloads
    /// such as an avatar or a manifest, never a bulk download; a host caps
    /// the body size and the wait.
    FetchUrl,
}

impl Action {
    pub(super) fn tag(self) -> u8 {
        match self {
            Action::ReadClipboard => 0,
            Action::WriteClipboard => 1,
            Action::OpenUrl => 2,
            Action::PickFile => 3,
            Action::PickFolder => 4,
            Action::OpenDirectory => 5,
            Action::FetchUrl => 6,
        }
    }

    pub(super) fn from_tag(tag: u8) -> core::result::Result<Self, DecodeError> {
        match tag {
            0 => Ok(Action::ReadClipboard),
            1 => Ok(Action::WriteClipboard),
            2 => Ok(Action::OpenUrl),
            3 => Ok(Action::PickFile),
            4 => Ok(Action::PickFolder),
            5 => Ok(Action::OpenDirectory),
            6 => Ok(Action::FetchUrl),
            tag => Err(DecodeError::InvalidTag {
                message: "os action",
                tag,
            }),
        }
    }
}
