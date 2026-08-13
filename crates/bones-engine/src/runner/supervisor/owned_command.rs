use bones_messages::extension_control::Command;

/// Owned command queued between bus dispatch and supervisor checks.
pub(crate) enum OwnedCommand {
    Load(String),
    Unload(String),
    Reload(String),
}

impl From<Command<'_>> for OwnedCommand {
    fn from(command: Command<'_>) -> Self {
        match command {
            Command::Load(message) => Self::Load(message.extension.to_string()),
            Command::Unload(message) => Self::Unload(message.extension.to_string()),
            Command::Reload(message) => Self::Reload(message.extension.to_string()),
        }
    }
}
