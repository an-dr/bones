use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

/// Links an entity to its rapier2d bodies — the `hecs` component side of
/// the physics integration. Position after each step is read back from
/// `body` into the entity's `Transform`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collider {
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

#[cfg(test)]
mod tests;
