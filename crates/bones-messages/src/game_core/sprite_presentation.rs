use super::Sprite;

/// A sprite's complete mutable presentation, applied without replacing its
/// entity, transform, or physics bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpritePresentation {
    pub sprite: Sprite,
    /// Source-sheet frames per row. `0` puts every frame on one row.
    pub frames_per_row: u32,
    /// Destination size in world pixels, independent of the source frame.
    pub draw_w: u32,
    pub draw_h: u32,
    /// Whether elapsed animation time wraps after the final frame.
    pub looping: bool,
    /// Advances even when the entity has no velocity, for idle or action
    /// animations. `false` preserves the original movement-gated behavior.
    pub advance_while_stopped: bool,
    pub flip_h: bool,
    pub flip_v: bool,
}
