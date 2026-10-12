//! `instance/*`: later launches of a single-instance application (ADR-035).
//!
//! When an application keeps one window per user, a second launch hands its
//! arguments to the running process and exits. The `instance` native module
//! publishes each such handoff here, so an extension or page reacts to a
//! later launch exactly as it would to startup arguments. The application's
//! own startup arguments are never published: it reads those itself.

mod launched;

pub use launched::Launched;

/// The bus endpoint name the `instance` native module registers under, and
/// the sender every `instance/*` message carries.
pub const ENDPOINT: &str = "instance";

#[cfg(test)]
mod tests;
