//! Everything concerned with a WASM extension's existence over time:
//! loading and dispatch (`host`), state-transition events (`lifecycle`),
//! saving/restoring its own state across a reload (`persistence`), and
//! reading files inside a granted directory (`files`) — as opposed to
//! `renderer`/`ui`/`audio`, which are about what an extension can
//! currently *do*.

pub mod files;
pub mod host;
pub mod lifecycle;
pub mod persistence;
