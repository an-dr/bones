//! Parses a Tiled `.tmx` map's `"Collision"` object layer into static
//! collider rectangles (ADR-019: `tiled` is parsing only — rendering the
//! tilemap itself stays `gfx/*`, out of this crate's scope).

use std::io::Cursor;
use std::path::Path;

use tiled::{Loader, ObjectShape};

const VIRTUAL_MAP_PATH: &str = "map.tmx";

/// One static collision rectangle, in world-space pixels, top-left origin
/// (Tiled's own coordinate convention) — the caller converts to whatever
/// origin its physics/render pipeline expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionRect {
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub half_h: f32,
}

/// Parses `tmx_bytes` and returns every rectangle object on an object layer
/// named `"Collision"`. A map with no such layer parses fine and yields no
/// rectangles — collision is opt-in per map, not assumed.
pub fn load_collision_rects(tmx_bytes: &[u8]) -> Result<Vec<CollisionRect>, String> {
    let mut loader = Loader::with_cache_and_reader(
        tiled::DefaultResourceCache::new(),
        |path: &Path| -> std::io::Result<_> {
            if path == Path::new(VIRTUAL_MAP_PATH) {
                Ok(Cursor::new(tmx_bytes.to_vec()))
            } else {
                Err(std::io::ErrorKind::NotFound.into())
            }
        },
    );
    let map = loader
        .load_tmx_map(VIRTUAL_MAP_PATH)
        .map_err(|err| err.to_string())?;

    let rects = map
        .layers()
        .filter(|layer| layer.name == "Collision")
        .filter_map(|layer| match layer.layer_type() {
            tiled::LayerType::Objects(objects) => Some(objects),
            _ => None,
        })
        .flat_map(|objects| objects.objects().collect::<Vec<_>>())
        .filter_map(|object| match object.shape {
            ObjectShape::Rect { width, height } => Some(CollisionRect {
                x: object.x + width / 2.0,
                y: object.y + height / 2.0,
                half_w: width / 2.0,
                half_h: height / 2.0,
            }),
            _ => None,
        })
        .collect();

    Ok(rects)
}

#[cfg(test)]
mod tests;
