/// The desktops whose look the ui module follows (ADR-049).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Desktop {
    Windows,
    Kde,
    Gnome,
    Cosmic,
}

impl Desktop {
    /// The desktop this process runs on; GNOME's look when it is none of
    /// the four, since every interface still needs one.
    pub(crate) fn detect() -> Self {
        if cfg!(windows) {
            return Desktop::Windows;
        }
        let value = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        from_xdg(&value).unwrap_or(Desktop::Gnome)
    }
}

/// Reads a `XDG_CURRENT_DESKTOP` value, whose entries are `:`-separated, as
/// in `ubuntu:GNOME`.
pub(crate) fn from_xdg(value: &str) -> Option<Desktop> {
    value
        .split(':')
        .find_map(|name| match name.to_ascii_uppercase().as_str() {
            "COSMIC" => Some(Desktop::Cosmic),
            "KDE" => Some(Desktop::Kde),
            "GNOME" => Some(Desktop::Gnome),
            _ => None,
        })
}
