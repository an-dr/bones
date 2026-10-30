//! `os/*`: the desktop capabilities a WASM extension cannot reach on its own.
//!
//! An extension has no OS API — no clipboard, no browser, no native file
//! dialog — because the sandbox that makes it safe to load also cuts it off
//! from the machine. This module is how it asks the trusted `os` native
//! module to act on its behalf, the same module-vs-extension trust split
//! `persistence/*` rests on.
//!
//! Every request carries a caller-chosen id. [`Reply`] repeats it alongside
//! the host-stamped requester; consumers match both. Legacy [`Result`]
//! requires application-wide ID coordination. The
//! native module answers off the bus thread — a file dialog blocks until the
//! user chooses, which must not stall the engine — so replies arrive in
//! whatever order the work finishes, never necessarily the order asked.
//!
//! A host is free to offer only some actions: an unimplemented one answers
//! with [`Result::error`] rather than failing to answer, so a guest never
//! waits forever on a capability this host does not have.

mod action;
mod reply;
mod request;
mod result;

pub use action::Action;
pub use reply::Reply;
pub use request::Request;
pub use result::Result;

/// The bus endpoint of the OS module.
pub const ENDPOINT: &str = "os";

#[cfg(test)]
mod tests;
