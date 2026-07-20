//! A no-mass, no-solver `PhysicsBackend` (ADR-021): velocity-integrated AABB
//! movement, overlaps resolved by a single minimum-translation-vector
//! push-apart per step — the "retro/arcade" feel ADR-021 asked for,
//! alongside `physics-rapier2d`'s full rigid-body simulation.

mod retro_backend;

pub use retro_backend::RetroBackend;
