/// How a [`super::Request`] ended.
///
/// Tags are part of the wire format: add new outcomes at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome<'a> {
    /// A release newer than the running version exists.
    Available { version: &'a str },
    /// The running version is the newest release, or no release is published
    /// for this platform.
    UpToDate,
    /// A newer release is staged; the launcher starts it next time.
    Staged { version: &'a str },
    /// The running build was installed. `fresh` is true when nothing was
    /// installed before, so the files went straight to the install location
    /// rather than into a version folder beside an existing install.
    Installed { fresh: bool },
    /// The version that ran before this one, and this one. `previous` is
    /// `None` on the first run of an install and for any run outside one.
    LastApplied {
        previous: Option<&'a str>,
        current: &'a str,
    },
    /// The action failed, with a human-readable reason.
    Failed { reason: &'a str },
}
