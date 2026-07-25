mod camera;
mod game_core;
mod graphics;
mod physics;
mod tiles;

pub use game_core::GameCore;
pub use graphics::{SpriteAnimation, SquareColor, Transform};
pub use physics::{
    BodyHandle, BodyKind, Collider, ColliderHandle, PhysicsBackend, PhysicsWorldKind,
    Rapier2dBackend, RetroBackend, WorldBody,
};
pub use tiles::{load_collision_rects, CollisionRect};
