//! Backend-agnostic physics contract (ADR-021): the `PhysicsBackend` trait
//! and its handle/kind types. No simulation lives here — `physics-rapier2d`
//! and `physics-retro` implement `PhysicsBackend`, and `game-core` drives
//! one instance per backend.

mod body_handle;
mod body_kind;
mod collider_handle;
mod physics_backend;

pub use body_handle::BodyHandle;
pub use body_kind::BodyKind;
pub use collider_handle::ColliderHandle;
pub use physics_backend::PhysicsBackend;
