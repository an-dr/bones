//! The facade crate (design/modules.md): the one name an embedder depends
//! on. Re-exports the kernel's public builder API; `app` is built on this
//! same facade, not on `bones-runner` directly, so an embedder using
//! `bones` has no access the shipped app lacks.

pub use bones_runner::{BuiltEngine, Engine, Supervisor};
