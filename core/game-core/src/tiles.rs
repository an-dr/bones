//! Tiled `.tmx` tilemap loading — collision geometry only, tile rendering
//! stays a `gfx/*` concern outside this crate.

mod tilemap;

pub use tilemap::{load_collision_rects, CollisionRect};
