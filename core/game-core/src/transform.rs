/// World-space position of an entity's **center** — the `hecs` component
/// every spawned entity carries. Center, not top-left: this is what
/// `rapier2d` colliders are centered on (`ColliderBuilder::cuboid`'s own
/// convention), so it's what `Collider`-bearing entities sync from their
/// rigid body's translation without an extra offset. `gfx::DrawSprite`/
/// `gfx::DrawRect` take a top-left `dst_x`/`dst_y` instead — callers
/// converting `Transform` to a draw command must subtract half the drawn
/// extent, the same conversion `publish_gfx` already applies for squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests;
