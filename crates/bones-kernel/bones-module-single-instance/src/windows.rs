//! The claim on Windows: a named pipe whose first instance is the claim.
//!
//! The name is `\\.\pipe\bones-single-instance-<user-sid>-<app_uid>`. Pipe names are
//! machine-wide, so the user's SID is what scopes an app_uid to one user, and a
//! client checks that the pipe's server runs as that user before handing it
//! anything.

mod attempt;
mod guard;
mod listener;
mod pipe;

pub(crate) use attempt::{allow_foreground, attempt};
pub(crate) use guard::Guard;
pub(crate) use listener::Listener;

/// A connection to the running instance.
pub(crate) type Stream = std::fs::File;
