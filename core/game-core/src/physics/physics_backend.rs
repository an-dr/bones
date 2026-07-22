use glam::Vec2;

use super::{BodyHandle, BodyKind, ColliderHandle, Shape};

/// One simulated physics world, independent of every other `PhysicsBackend`
/// instance — the seam ADR-021 cuts rapier2d out from behind. `game-core`
/// owns one instance per backend kind and drives each one fully every
/// tick; nothing here assumes it is the only world running, or that its
/// bodies interact with another world's.
pub trait PhysicsBackend {
    /// Adds a body with an axis-aligned box collider centered on `position`
    /// with the given half-extents, returning the handles `game-core`
    /// stores to address it on every later call. A backend with no real
    /// collider concept (unlikely, but not assumed away) may still return
    /// a distinct `ColliderHandle` — the two handles are never assumed
    /// equal or interchangeable by callers.
    ///
    /// A thin default over `spawn_shaped_body` with `Shape::Rect` — the
    /// overwhelming majority of callers want a box and don't need to name
    /// `Shape` at every call site.
    fn spawn_body(
        &mut self,
        position: Vec2,
        half_extents: Vec2,
        kind: BodyKind,
    ) -> (BodyHandle, ColliderHandle) {
        self.spawn_shaped_body(position, half_extents, Shape::Rect, kind)
    }

    /// Adds a body with the given `Shape`, sized by `half_extents` either
    /// way (a `Triangle`'s base/height both come from the same box a
    /// `Rect` of these half-extents would occupy). A backend with no real
    /// non-`Rect` collider concept may approximate any other `Shape` as
    /// its own `Rect` bounding box instead of rejecting it — see the
    /// backend's own docs for whether it does.
    fn spawn_shaped_body(
        &mut self,
        position: Vec2,
        half_extents: Vec2,
        shape: Shape,
        kind: BodyKind,
    ) -> (BodyHandle, ColliderHandle);

    /// Removes a body and its collider. A no-op if `body` names no body in
    /// this world (already removed, never valid here, or valid only in a
    /// different `PhysicsBackend` instance).
    fn remove_body(&mut self, body: BodyHandle);

    /// How many bodies currently exist in this world.
    fn body_count(&self) -> usize;

    /// Directly sets a body's linear velocity — the mechanism a caller
    /// (an extension reading `input/*`) drives movement with. A no-op if
    /// `body` names no body in this world.
    fn set_velocity(&mut self, body: BodyHandle, velocity: Vec2);

    /// A body's current linear velocity, or `None` if `body` names no body
    /// in this world.
    fn velocity(&self, body: BodyHandle) -> Option<Vec2>;

    /// A body's current position, or `None` if `body` names no body in
    /// this world.
    fn body_translation(&self, body: BodyHandle) -> Option<Vec2>;

    /// Whether `body` is a `BodyKind::Kinematic` body — moves exactly as
    /// commanded and is never itself pushed, so contact-clamping logic
    /// (undoing a commanded velocity's inward component after a step) has
    /// nothing to clamp on it. `false` if `body` names no body in this
    /// world.
    fn is_kinematic(&self, body: BodyHandle) -> bool;

    /// Directly overwrites a body's position and velocity — used to snap a
    /// lower-priority world's copy of a multi-world entity to the
    /// authoritative position each tick (ADR-021), not by ordinary
    /// gameplay code. A no-op if `body` names no body in this world.
    fn set_body_state(&mut self, body: BodyHandle, position: Vec2, velocity: Vec2);

    /// Advances the simulation by `dt` seconds.
    fn step(&mut self, dt: f32);

    /// Drains every new contact-start pair produced since the last call —
    /// contact separations are not reported, only new touches.
    fn drain_collision_starts(&mut self) -> Vec<(ColliderHandle, ColliderHandle)>;

    /// Whether two colliders are actually touching right now (not merely
    /// within a backend's speculative-contact prediction margin, if it has
    /// one). `false` if either handle names no collider in this world.
    fn has_real_contact(&self, a: ColliderHandle, b: ColliderHandle) -> bool;

    /// Every real (see `has_real_contact`) contact normal touching
    /// `collider` right now, each pointing away from `collider` — the
    /// direction a commanded velocity must not have a component along, or
    /// it re-drives the body into whatever it's already touching. Empty if
    /// `collider` names no collider in this world, or has no real contacts.
    fn contact_normals(&self, collider: ColliderHandle) -> Vec<Vec2>;
}

#[cfg(test)]
mod tests;
