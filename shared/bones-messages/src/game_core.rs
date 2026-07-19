//! Typed `game-core/*` commands shared by extensions and the game-core
//! module.

mod command;
mod load_tilemap;
mod set_velocity;
mod spawn_entity;

pub use command::Command;
pub use load_tilemap::LoadTilemap;
pub use set_velocity::SetVelocity;
pub use spawn_entity::SpawnEntity;

#[cfg(test)]
mod tests;
