use ab_glyph::{point, Font as _, FontRef, PxScale, ScaleFont};

// `text`/`size` come from an untrusted extension's `DrawText` payload.
// Unclamped, a hostile or buggy extension could request a rasterization
// buffer proportional to chars * size^2 — large enough to hang or OOM the
// render phase, which (unlike an extension) has no time budget
// (design/modules.md's trust model). Both caps are far larger than any
// real one-line label, so neither clips an intentional draw.
const MAX_TEXT_SIZE_PX: f32 = 256.0;
const MAX_TEXT_CHARS: usize = 512;

/// Rasterizes one line of `text` at `size` px into a straight-alpha RGBA8
/// buffer sized to its own bounding box, tinted `color`. No wrapping, no
/// glyph/atlas cache — text draws are rare enough per frame that
/// re-rasterizing each one is simpler and not worth the cache-invalidation
/// cost (same reasoning as the sprite/shape draws' lack of a cache).
pub(crate) fn rasterize_text(
    font: &FontRef,
    text: &str,
    size: f32,
    color: (u8, u8, u8, u8),
) -> (u32, u32, Vec<u8>) {
    let size = size.clamp(1.0, MAX_TEXT_SIZE_PX);
    let scaled = font.as_scaled(PxScale::from(size));
    let ascent = scaled.ascent();
    let height = scaled.height().ceil().max(1.0) as u32;

    let mut glyphs = Vec::new();
    let mut cursor = 0.0f32;
    for c in text.chars().take(MAX_TEXT_CHARS) {
        let id = scaled.glyph_id(c);
        glyphs.push(id.with_scale_and_position(PxScale::from(size), point(cursor, ascent)));
        cursor += scaled.h_advance(id);
    }
    let width = cursor.ceil().max(1.0) as u32;

    let (r, g, b, a) = color;
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    for glyph in glyphs {
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px as u32 >= width || py as u32 >= height {
                    return;
                }
                let idx = ((py as u32 * width + px as u32) * 4) as usize;
                buffer[idx] = r;
                buffer[idx + 1] = g;
                buffer[idx + 2] = b;
                buffer[idx + 3] = (a as f32 * coverage).round() as u8;
            });
        }
    }
    (width, height, buffer)
}
