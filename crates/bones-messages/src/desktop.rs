//! `desktop/*`: registering one application with the desktop (ADR-036).
//!
//! Which application, under which names and with which executable, is fixed
//! by whoever composed the engine. A request only chooses the action: a guest
//! that could name the executable could register any program as a folder
//! verb the shell runs for the user.
//!
//! Every request carries a caller-chosen id and the result repeats it. The
//! native module works off the frame thread, so results arrive in the order
//! the work finishes.

mod action;
mod outcome;
mod request;
mod result;

pub use action::Action;
pub use outcome::Outcome;
pub use request::Request;
pub use result::Result;

/// The bus endpoint name the `desktop` native module registers under, and
/// the sender of every `desktop/result`.
pub const ENDPOINT: &str = "desktop";

#[cfg(test)]
mod tests;
