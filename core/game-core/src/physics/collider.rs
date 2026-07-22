use super::{PhysicsWorldKind, Shape, WorldBody};

/// Links an entity to its physics body in every world it's registered in
/// (ADR-021: an entity may be in more than one) — the `hecs` component
/// side of the physics integration, addressed through
/// `physics::PhysicsBackend`'s opaque handles rather than any specific
/// engine's types. `half_w`/`half_h`/`shape` duplicate the collider's own
/// extent/shape (rather than querying a backend for it at render time) —
/// a `SquareColor` entity needs its size and shape to draw, and this is
/// the cheap place to keep it, shared across every world since a spawn
/// request carries one extent/shape for all of them.
// No `Eq`: `half_w`/`half_h` are `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct Collider {
    // Never empty: `GameCore::spawn_entity` only attaches a `Collider` at
    // all when the requested worlds produced at least one body.
    pub bodies: Vec<WorldBody>,
    pub half_w: f32,
    pub half_h: f32,
    pub shape: Shape,
}

impl Collider {
    /// The body registered in `world`, if this entity is registered there
    /// at all.
    pub fn in_world(&self, world: PhysicsWorldKind) -> Option<&WorldBody> {
        self.bodies.iter().find(|wb| wb.world == world)
    }

    /// The highest-priority world (`PhysicsWorldKind::PRIORITY`) this
    /// entity is registered in — the world its drawn/exposed `Transform`
    /// is read from every tick. Never `None`: `bodies` is never empty.
    pub fn primary(&self) -> &WorldBody {
        PhysicsWorldKind::PRIORITY
            .iter()
            .find_map(|world| self.in_world(*world))
            .expect("Collider::bodies is never empty")
    }
}

#[cfg(test)]
mod tests;
