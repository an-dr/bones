use std::io::Cursor;
use std::path::Path;

use tiled::{LayerType, Loader, TileLayer};

const VIRTUAL_MAP_PATH: &str = "map.tmx";

/// One tile to draw, resolved from a `.tmx`'s `"Ground"` tile layer plus
/// whichever tileset a caller-supplied resolver could name a sprite for —
/// a caller-facing `gfx::DrawSprite`-shaped result (already carrying the
/// caller's own `sprite_id`), not `tiled`'s own GID/tileset-index
/// representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileDraw {
    pub sprite_id: u32,
    pub dst_x: i32,
    pub dst_y: i32,
    /// Both the drawn size and the source crop size — square, since every
    /// tileset this crate resolves a draw from is assumed to use the same
    /// width and height per tile (Tiled allows a non-square tile grid, but
    /// nothing here needs it).
    pub size: u32,
    pub src_x: i32,
    pub src_y: i32,
}

/// Parses `tmx_bytes` and returns one `TileDraw` per non-empty cell on a
/// tile layer named `"Ground"`, resolving each cell's tileset through
/// `sprite_id_for_tileset` (keyed by the tileset's own `name` attribute in
/// the `.tmx`) — a cell whose tileset resolves to `None` is silently
/// skipped, not an error, so a map can reference a tileset a particular
/// caller doesn't care to draw. A map with no `"Ground"` layer, or no
/// tile layer with that name, parses fine and yields no draws. An
/// infinite `"Ground"` layer is also skipped (silently): this function
/// only understands the finite, fixed-size layer shape every caller in
/// this codebase actually authors.
pub fn load_tile_draws(
    tmx_bytes: &[u8],
    sprite_id_for_tileset: impl Fn(&str) -> Option<u32>,
) -> Result<Vec<TileDraw>, String> {
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

    let mut draws = Vec::new();
    for layer in map.layers().filter(|layer| layer.name == "Ground") {
        let LayerType::Tiles(TileLayer::Finite(finite)) = layer.layer_type() else {
            continue;
        };
        for y in 0..finite.height() as i32 {
            for x in 0..finite.width() as i32 {
                let Some(layer_tile) = finite.get_tile(x, y) else {
                    continue;
                };
                let tileset = layer_tile.get_tileset();
                let Some(sprite_id) = sprite_id_for_tileset(&tileset.name) else {
                    continue;
                };
                let columns = tileset.columns.max(1);
                let col = layer_tile.id() % columns;
                let row = layer_tile.id() / columns;
                draws.push(TileDraw {
                    sprite_id,
                    dst_x: x * map.tile_width as i32,
                    dst_y: y * map.tile_height as i32,
                    size: tileset.tile_width,
                    src_x: (tileset.margin + col * (tileset.tile_width + tileset.spacing)) as i32,
                    src_y: (tileset.margin + row * (tileset.tile_height + tileset.spacing)) as i32,
                });
            }
        }
    }
    Ok(draws)
}

/// The map's total pixel extent (`width`/`height` in tiles times
/// `tile_width`/`tile_height`), for camera-follow clamping
/// (`EntityOp::SetCameraFollow`) — `None` for an unparseable `.tmx`, the
/// same "ignore rather than fail the module" contract `load_tile_draws`
/// and `load_collision_rects` already follow. A third small reparse of
/// `tmx_bytes` rather than threading this out of `load_tile_draws`'s own
/// parse: consistent with this module's existing one-parse-per-concern
/// functions, and the `.tmx` files this loads are small, load-time-only
/// parses, not a per-tick cost.
pub fn map_pixel_size(tmx_bytes: &[u8]) -> Option<(f32, f32)> {
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
    let map = loader.load_tmx_map(VIRTUAL_MAP_PATH).ok()?;
    Some((
        (map.width * map.tile_width) as f32,
        (map.height * map.tile_height) as f32,
    ))
}

#[cfg(test)]
mod tests;
