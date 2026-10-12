/// Opaque handle to a collider in one `PhysicsBackend` world — same
/// backend-scoped-identity contract as `BodyHandle`, kept as a distinct
/// type since a body and its attached collider are different concepts to
/// every backend (rapier2d's `RigidBodyHandle`/`ColliderHandle` split) and
/// to `game-core`'s own call sites (contact/collision queries are always
/// collider-addressed, movement/removal always body-addressed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderHandle(pub u64);

#[cfg(test)]
mod tests;
