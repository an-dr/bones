/// Opaque handle to a body in one `PhysicsBackend` world — backend-internal
/// identity, meaningless across two different backend instances (ADR-021:
/// each world assigns its own). A `PhysicsBackend` implementation
/// constructs these from its own internal index/generation and hands them
/// out from `spawn_body`; a caller (`game-core`) only ever stores and
/// passes one back, never builds or inspects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub u64);

#[cfg(test)]
mod tests;
