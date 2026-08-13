/// One tileset's image bytes, supplied alongside a `LoadTilemap` so
/// `game-core` can register it (`gfx::LoadSprite`) and draw tiles from it.
/// Matched to the `.tmx`'s own embedded `<tileset name="...">` by `name` —
/// a tileset the `.tmx` references with no matching `TilesetImage` here is
/// parsed (so collision geometry still works) but never drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilesetImage<'a> {
    /// Must match a `.tmx` `<tileset>` element's own `name` attribute.
    pub name: &'a str,
    /// The `gfx::LoadSprite`/`gfx::DrawSprite` id this image registers
    /// under — caller-assigned, the same namespace `EntityOp::Spawn`'s
    /// `Sprite::sprite_id` uses.
    pub sprite_id: u32,
    /// Raw image bytes (PNG), the same format `gfx::LoadSprite` expects.
    pub png_bytes: &'a [u8],
}
