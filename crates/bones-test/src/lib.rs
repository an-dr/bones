//! Test a built extension on a headless kernel: send it input, step frames,
//! and check what it published.
//!
//! - [`build`] compiles an extension crate for `wasm32-wasip2`.
//! - [`App`] loads one `.wasm` into the real extension host and records every
//!   message on the bus, its draw commands included.
//! - With the `render` feature, [`App::rendered`] adds the real renderer in
//!   an offscreen window, and `App::screenshot` returns its pixels.
//!
//! See the crate README for the layout a product uses.

#![warn(missing_docs)]

mod app;
mod build;
mod recorder;
mod recording;
#[cfg(feature = "render")]
mod screenshot;

pub use app::App;
pub use build::build;
#[cfg(feature = "render")]
pub use screenshot::Screenshot;

/// The kernel, for configuring one with [`App::with_kernel`] without a
/// second dependency.
pub use bones_kernel as kernel;

/// The core message vocabulary, for building input and reading output.
pub use bones_messages as messages;

/// What can go wrong building or loading an extension, as a message to act on.
pub type Error = String;
