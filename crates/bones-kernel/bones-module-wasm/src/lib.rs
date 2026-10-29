//! The extension host as an opt-in native module (ADR-040).
//!
//! Loads `.wasm` components against the `bones:extension` WIT world, gives
//! each its own bus endpoint, and owns everything about an extension's life:
//! the watchdog and quarantine (ADR-007), hot reload, runtime activation, and
//! `core/lifecycle` events. The bus belongs to the kernel infrastructure,
//! so an extension reaches every module, and every module reaches it, without
//! either knowing the other is WASM.
//!
//! - `host` loads one component and dispatches to it.
//! - `loading` attaches a component to the bus and discovers files.
//! - `supervisor` quarantines, reloads, and applies runtime commands.
//! - `lifecycle` publishes state transitions.
//! - [`Permissions`] is one extension's deny-list (ADR-042).
//! - [`Wasm`] is the module that composes them, and the only type most
//!   callers name.

mod contract;
mod permissions;
mod wasm;

pub mod host;
pub mod lifecycle;
pub mod loading;
pub mod supervisor;

pub use permissions::Permissions;
pub use wasm::Wasm;
