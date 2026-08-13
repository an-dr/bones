//! The kernel: the host crates that are always present and never orchestrate
//! an optional native module, merged into one compiled crate (ADR-030).
//! `bones-engine` depends on this and the optional `bones-module-*` crates,
//! and holds the orchestration (the former `runner`) itself, since a crate
//! that both leaf infrastructure and its own consumer depend on cannot also
//! depend back on that consumer without a cycle.

mod contract;

pub mod bus;
pub mod logging;
pub mod wasm_extensions;

#[cfg(feature = "platform")]
pub mod platform;
