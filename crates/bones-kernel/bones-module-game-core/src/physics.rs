//! Physics: the backend-agnostic `PhysicsBackend` trait (ADR-021/ADR-022),
//! its `Rapier2dBackend`/`RetroBackend` implementations, and the ECS-facing
//! `Collider`/`WorldBody` types that address a body across one or more
//! worlds.

mod body_handle;
mod body_kind;
mod collider;
mod collider_handle;
mod physics_backend;
mod physics_world_kind;
mod rapier2d_backend;
mod retro_backend;
mod shape;
mod world_body;

pub use body_handle::BodyHandle;
pub use body_kind::BodyKind;
pub use collider::Collider;
pub use collider_handle::ColliderHandle;
pub use physics_backend::PhysicsBackend;
pub use physics_world_kind::PhysicsWorldKind;
pub use rapier2d_backend::Rapier2dBackend;
pub use retro_backend::RetroBackend;
pub use shape::Shape;
pub use world_body::WorldBody;
