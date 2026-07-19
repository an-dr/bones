//! Everything concerned with a WASM extension's existence over time:
//! loading and dispatch (`host`), state-transition events (`lifecycle`),
//! and saving/restoring its own state across a reload (`persistence`) —
//! as opposed to `renderer`/`ui`/`audio`, which are about what an
//! extension can currently *do*.

pub mod host;
pub mod lifecycle;
pub mod persistence;
