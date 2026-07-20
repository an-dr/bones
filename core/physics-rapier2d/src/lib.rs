//! `rapier2d`-backed `PhysicsBackend` (ADR-021, ADR-019's original
//! physics/collision choice) — a full rigid-body simulation: mass,
//! impulses, a real solver.

mod rapier2d_backend;

pub use rapier2d_backend::Rapier2dBackend;
