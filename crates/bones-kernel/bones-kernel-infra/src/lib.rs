//! The kernel infrastructure: the host code that is always present and never orchestrates
//! an optional native module, merged into one compiled crate (ADR-030).
//! `bones-kernel` depends on this and the optional `bones-module-*` crates.
//!
//! Everything here is module-agnostic: the frame loop and persistence run
//! with zero native modules registered. Extensions are hosted by the opt-in
//! `bones-module-wasm` crate (ADR-040), which builds on this one; nothing here
//! depends on a WASM runtime.
//! What names a concrete module — the builder that composes them — stays in
//! `bones-kernel`, which is the one crate that may depend on both this and
//! the `bones-module-*` crates without a cycle.

pub mod bus;
pub mod display;
pub mod draw_target;
pub mod exit;
pub mod files;
pub mod frame_control;
pub mod logging;
pub mod persistence;
pub mod runner;
pub mod window_control;

#[cfg(feature = "platform")]
pub mod platform;
