//! Typed `game-core/*` messages shared by extensions and the game-core
//! module. `EntityOp` is the open/closed extension point for per-entity
//! operations (spawn, set-velocity, despawn, and future ones) — new ops
//! extend that enum rather than adding a new bus topic, the same pattern
//! `ui::Widget` uses for `ui/spec`. `LoadTilemap` stays its own topic: a
//! one-shot asset load, not a per-entity operation.

mod entity_op;
mod entity_op_message;
mod load_tilemap;

pub use entity_op::{EntityOp, Sprite};
pub use entity_op_message::EntityOpMessage;
pub use load_tilemap::LoadTilemap;

#[cfg(test)]
mod tests;
