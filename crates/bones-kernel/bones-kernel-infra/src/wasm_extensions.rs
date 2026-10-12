//! Everything concerned with a WASM extension's existence over time:
//! discovery and the attach ceremony (`loading`), loading and dispatch
//! (`host`), fault quarantine and hot reload (`supervisor`),
//! state-transition events (`lifecycle`), saving/restoring its own state
//! across a reload (`persistence`), and reading files inside a granted
//! directory (`files`) — as opposed to `renderer`/`ui`/`audio`, which are
//! about what an extension can currently *do*.

pub mod files;
pub mod host;
pub mod lifecycle;
pub mod loading;
pub mod persistence;
pub mod supervisor;
