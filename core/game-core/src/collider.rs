use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

/// Links an entity to its rapier2d bodies — the `hecs` component side of
/// the physics integration. Position after each step is read back from
/// `body` into the entity's `Transform`. `half_w`/`half_h` duplicate the
/// box collider's own extent (rather than querying rapier2d's `Collider`
/// for its shape at render time) — a `SquareColor` entity needs its size
/// to draw, and this is the cheap place to keep it.
// No `Eq`: `half_w`/`half_h` are `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
    pub half_w: f32,
    pub half_h: f32,
}

#[cfg(test)]
mod tests;
