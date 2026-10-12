//! The kernel infrastructure: the host code that is always present and never orchestrates
//! an optional native module, merged into one compiled crate (ADR-030).
//! `bones-kernel` depends on this and the optional `bones-module-*` crates.
//!
//! Everything here is module-agnostic: the frame loop, extension loading,
//! and extension supervision all run with zero native modules registered.
//! What names a concrete module — the builder that composes them — stays in
//! `bones-kernel`, which is the one crate that may depend on both this and
//! the `bones-module-*` crates without a cycle.

mod contract;

pub mod bus;
pub mod draw_target;
pub mod logging;
pub mod runner;
pub mod wasm_extensions;
pub mod window_control;

#[cfg(feature = "platform")]
pub mod platform;
