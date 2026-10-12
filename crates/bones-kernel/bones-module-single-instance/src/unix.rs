//! The claim on Unix: a domain socket that something is accepting on.
//!
//! The socket lives in `$XDG_RUNTIME_DIR`, which is the user's own, or in a
//! private per-user folder under the temporary directory when that is unset.
//! A socket file whose owner crashed refuses connections, which is how a
//! stale claim is told apart from a live one; a lock file beside it keeps two
//! launches from both deciding the claim is stale and both taking it.

mod attempt;
mod guard;
mod listener;
mod runtime_dir;

pub(crate) use attempt::{allow_foreground, attempt};
pub(crate) use guard::Guard;
pub(crate) use listener::Listener;

/// A connection to the running instance.
pub(crate) type Stream = std::os::unix::net::UnixStream;
