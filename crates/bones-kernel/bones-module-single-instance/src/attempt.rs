use crate::platform::{Listener, Stream};

/// One try at the claim, as the platform saw it.
pub(crate) enum Attempt {
    /// The endpoint was free and is now this process's.
    Claimed(Listener),
    /// Another process holds the endpoint and accepted a connection to it.
    Taken(Stream),
    /// The endpoint belonged to a process that has gone; try again.
    Stale,
    /// The endpoint exists but cannot be handed to: another account holds
    /// it, or it stayed busy past the timeout. Start as an instance of its
    /// own.
    Unavailable,
}
