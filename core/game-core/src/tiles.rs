//! Tiled `.tmx` tilemap loading: collision geometry (`tilemap`) and a
//! `"Ground"` tile layer's visual tiles (`tile_draw`), resolved to
//! caller-assigned sprite ids via a name-keyed resolver the caller
//! supplies — this crate never opens an image file itself, only parses
//! the `.tmx`'s own tileset/layer data (`tiled` crate).

mod tile_draw;
mod tilemap;

pub use tile_draw::{load_tile_draws, TileDraw};
pub use tilemap::{load_collision_rects, CollisionRect};
