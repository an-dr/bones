//! Desktop registration (ADR-036): an entry in the system's application list
//! and "Open in" on folders, per user, and removing them again.
//!
//! - **Called directly** — [`register`], [`unregister`], [`is_registered`] —
//!   from an application's `--install` step, which runs without a kernel.
//! - **Over the bus** — [`DesktopModule`] — for a page or guest, for the one
//!   [`Integration`] the composition fixes: a guest chooses the action, never
//!   what is registered, because registration writes a command the shell runs.
//!
//! The application's own files are never touched, so an unregistered
//! application is simply portable.

mod desktop_module;
mod integration;
#[cfg(target_os = "linux")]
mod linux;
mod locations;
mod registration;
#[cfg(windows)]
mod windows;
mod windows_verbs;

pub use desktop_module::DesktopModule;
pub use integration::Integration;
pub use locations::Locations;
pub use registration::{is_registered, register, unregister};

#[cfg(test)]
mod tests;
