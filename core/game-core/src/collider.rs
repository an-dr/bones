use physics::{BodyHandle, ColliderHandle};

/// Links an entity to its physics body — the `hecs` component side of the
/// physics integration (ADR-021: backend-agnostic, addressed through
/// `physics::PhysicsBackend`'s opaque handles rather than a specific
/// engine's types). Position after each step is read back from `body`
/// into the entity's `Transform`. `half_w`/`half_h` duplicate the box
/// collider's own extent (rather than querying the backend for its shape
/// at render time) — a `SquareColor` entity needs its size to draw, and
/// this is the cheap place to keep it.
// No `Eq`: `half_w`/`half_h` are `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    pub body: BodyHandle,
    pub collider: ColliderHandle,
    pub half_w: f32,
    pub half_h: f32,
}

#[cfg(test)]
mod tests;
