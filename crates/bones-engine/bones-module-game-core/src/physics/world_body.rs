use super::{BodyHandle, ColliderHandle, PhysicsWorldKind};

/// Links an entity to its physics body in one world it's registered in —
/// see `Collider` for the (possibly multi-world) whole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBody {
    pub world: PhysicsWorldKind,
    pub body: BodyHandle,
    pub collider: ColliderHandle,
}

#[cfg(test)]
mod tests;
