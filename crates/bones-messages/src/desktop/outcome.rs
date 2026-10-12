/// How a [`super::Request`] ended.
///
/// Tags are part of the wire format: add new outcomes at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome<'a> {
    /// The action completed; carries whether every part of the registration
    /// is present now. A status request answers with this too.
    Registered(bool),
    /// The action failed, with a human-readable reason.
    Failed(&'a str),
}
