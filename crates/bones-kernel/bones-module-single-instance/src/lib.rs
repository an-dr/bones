//! Single instance per user (ADR-035), in two halves that run at different
//! times.
//!
//! - **Before the kernel:** [`claim`] runs in an application's `main`, before
//!   a window opens, because a second instance that has opened one has
//!   already failed. It makes this process the instance for its `app_uid`, or
//!   hands the launch's arguments to the process that is. The claim is a
//!   per-user local endpoint — a named pipe on Windows, a Unix domain socket
//!   elsewhere — so a crashed instance leaves nothing a later launch trusts.
//! - **Inside the kernel:** [`InstanceModule`] publishes each later launch as
//!   `instance/launched` and raises the window.

mod app_uid;
mod attempt;
mod claim;
mod handoff;
mod instance_module;
mod primary;
mod wire;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
use windows as platform;

pub use claim::{claim, Claim};
pub use handoff::Handoff;
pub use instance_module::InstanceModule;
pub use primary::Primary;

#[cfg(test)]
mod tests;
