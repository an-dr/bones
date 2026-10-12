//! `upgrader/*`: self-update on a page's or guest's behalf (ADR-037).
//!
//! Where releases come from is fixed by whoever composed the engine. A
//! request only chooses the action: a guest that could name the manifest URL
//! could make the host stage any binary, and the checksum arrives from the
//! same manifest, so it would be no defence.
//!
//! Every request carries a caller-chosen id and the result repeats it. The
//! native module downloads off the frame thread, so results arrive in the
//! order the work finishes.

mod action;
mod outcome;
mod request;
mod result;

pub use action::Action;
pub use outcome::Outcome;
pub use request::Request;
pub use result::Result;

/// The bus endpoint name the `upgrader` native module registers under, and
/// the sender of every `upgrader/result`.
pub const ENDPOINT: &str = "upgrader";

#[cfg(test)]
mod tests;
