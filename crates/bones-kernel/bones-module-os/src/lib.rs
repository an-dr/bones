//! The `os` native module: desktop capabilities on behalf of a sandboxed guest.
//!
//! An extension has no clipboard, no browser and no file dialog, because the
//! sandbox that makes it safe to load also cuts it off from the machine. This
//! module is the trusted side of that split: it subscribes to `os/request`,
//! performs the action, and answers with caller identity on os/reply and on `os/result`.
//!
//! Work runs on at most 16 concurrent request threads, with no waiting queue. A file dialog blocks until the
//! user chooses, and a fetch until the remote answers; neither may stall the
//! engine, so replies arrive when the work finishes rather than in the order
//! asked. The requester and request ID correlate each os/reply completion.
//!
//! [`OsBackend`] is the seam. The module owns the bus protocol and nothing
//! else, so a host can swap in a backend of its own -- a test one that opens
//! no dialogs, or a platform one this crate does not cover -- without
//! reimplementing any of the plumbing.

mod os_backend;
mod os_module;
mod system_os_backend;

pub use os_backend::OsBackend;
pub use os_module::{OsModule, MAX_IN_FLIGHT};
pub use system_os_backend::{SystemOsBackend, FETCH_TIMEOUT, MAX_FETCH_BYTES};

#[cfg(test)]
mod tests;
