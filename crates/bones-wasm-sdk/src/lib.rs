//! Everything needed to write a bones extension in Rust, as one dependency.
//!
//! An extension is a WASM guest: it never touches the OS, never renders, and
//! reaches the engine only through the calls in [`bindings`]. This crate
//! carries the `bones:core` WIT package, generates the guest bindings from it,
//! and re-exports the shared message vocabulary, so a guest crate declares one
//! dependency instead of hand-copying `core.wit` into its own tree.
//!
//! ```ignore
//! use bones_wasm_sdk::bindings::bones::core::host_api::{log, subscribe, Level};
//! use bones_wasm_sdk::Guest;
//!
//! struct Component;
//!
//! impl Guest for Component {
//!     fn init() { subscribe("core/tick"); log(Level::Info, "init"); }
//!     fn shutdown() {}
//!     fn on_tick(_dt: f32) {}
//!     fn on_message(_t: String, _s: String, _p: Vec<u8>) -> Option<Vec<u8>> { None }
//! }
//!
//! bones_wasm_sdk::export!(Component);
//! ```
//!
//! Guests in other languages do not use this crate. They take `wit/core.wit`
//! and the message wire format directly — see `wit/README.md`.

/// Guest bindings generated from the `bones:core` WIT package.
///
/// Generated inside a module rather than at the crate root on purpose:
/// `pub_export_macro` marks the export macros `#[macro_export]`, which hoists
/// them to the crate root, and the sibling `pub use` would then collide with
/// them there (`E0255`). A module keeps the two apart.
pub mod bindings {
    wit_bindgen::generate!({
        path: "../../wit",
        world: "extension",
        pub_export_macro: true,
        default_bindings_module: "bones_wasm_sdk::bindings",
    });
}

/// The trait an extension implements: `init`, `shutdown`, `on_tick`,
/// `on_message`.
pub use bindings::Guest;

/// Wires a type implementing [`Guest`] up as the component's exports.
///
/// Must be invoked in the `cdylib` crate that becomes the component, which is
/// why it is a macro rather than a blanket impl.
pub use bindings::export;

/// The typed core messages and their payload codecs, shared byte-for-byte with
/// the native host.
pub use bones_messages as messages;

#[cfg(feature = "game-ui")]
pub mod game_ui;
