use std::collections::HashMap;

use super::*;

/// A minimal in-memory `PhysicsBackend` used only to prove the trait's
/// shape is implementable and object-safe (`game-core` needs to hold
/// several different backends behind `Box<dyn PhysicsBackend>`) — not a
/// real simulation, so `step`/contacts are no-ops.
#[derive(Default)]
struct FakeBackend {
    next_id: u64,
    positions: HashMap<u64, Vec2>,
    velocities: HashMap<u64, Vec2>,
    kinds: HashMap<u64, BodyKind>,
}

impl PhysicsBackend for FakeBackend {
    fn spawn_shaped_body(
        &mut self,
        position: Vec2,
        _half_extents: Vec2,
        _shape: Shape,
        kind: BodyKind,
    ) -> (BodyHandle, ColliderHandle) {
        let id = self.next_id;
        self.next_id += 1;
        self.positions.insert(id, position);
        self.velocities.insert(id, Vec2::ZERO);
        self.kinds.insert(id, kind);
        (BodyHandle(id), ColliderHandle(id))
    }

    fn remove_body(&mut self, body: BodyHandle) {
        self.positions.remove(&body.0);
        self.velocities.remove(&body.0);
        self.kinds.remove(&body.0);
    }

    fn body_count(&self) -> usize {
        self.positions.len()
    }

    fn set_velocity(&mut self, body: BodyHandle, velocity: Vec2) {
        if let Some(v) = self.velocities.get_mut(&body.0) {
            *v = velocity;
        }
    }

    fn velocity(&self, body: BodyHandle) -> Option<Vec2> {
        self.velocities.get(&body.0).copied()
    }

    fn body_translation(&self, body: BodyHandle) -> Option<Vec2> {
        self.positions.get(&body.0).copied()
    }

    fn set_body_state(&mut self, body: BodyHandle, position: Vec2, velocity: Vec2) {
        if let Some(p) = self.positions.get_mut(&body.0) {
            *p = position;
            self.velocities.insert(body.0, velocity);
        }
    }

    fn is_kinematic(&self, body: BodyHandle) -> bool {
        self.kinds.get(&body.0) == Some(&BodyKind::Kinematic)
    }

    fn step(&mut self, _dt: f32) {}

    fn drain_collision_starts(&mut self) -> Vec<(ColliderHandle, ColliderHandle)> {
        Vec::new()
    }

    fn has_real_contact(&self, _a: ColliderHandle, _b: ColliderHandle) -> bool {
        false
    }

    fn contact_normals(&self, _collider: ColliderHandle) -> Vec<Vec2> {
        Vec::new()
    }
}

#[test]
fn a_fake_backend_is_usable_behind_a_trait_object() {
    let mut backend: Box<dyn PhysicsBackend> = Box::new(FakeBackend::default());

    let (body, collider) = backend.spawn_body(Vec2::new(1.0, 2.0), Vec2::new(0.5, 0.5), BodyKind::Dynamic);
    assert_eq!(backend.body_translation(body), Some(Vec2::new(1.0, 2.0)));
    assert!(!backend.is_kinematic(body));

    backend.set_velocity(body, Vec2::new(3.0, 0.0));
    assert_eq!(backend.velocity(body), Some(Vec2::new(3.0, 0.0)));

    backend.set_body_state(body, Vec2::new(5.0, 5.0), Vec2::ZERO);
    assert_eq!(backend.body_translation(body), Some(Vec2::new(5.0, 5.0)));

    assert!(backend.contact_normals(collider).is_empty());

    backend.remove_body(body);
    assert_eq!(backend.body_translation(body), None);
}

#[test]
fn spawn_body_defaults_to_shape_rect() {
    // `spawn_body`'s default impl delegates to `spawn_shaped_body` with
    // `Shape::Rect` — proven indirectly here since `FakeBackend` doesn't
    // distinguish shapes itself, just that the default method compiles and
    // produces a usable body through the trait object.
    let mut backend: Box<dyn PhysicsBackend> = Box::new(FakeBackend::default());
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    assert_eq!(backend.body_translation(body), Some(Vec2::ZERO));
}

#[test]
fn spawn_shaped_body_accepts_a_triangle_shape() {
    let mut backend: Box<dyn PhysicsBackend> = Box::new(FakeBackend::default());
    let (body, _) =
        backend.spawn_shaped_body(Vec2::ZERO, Vec2::new(1.0, 1.0), Shape::Triangle, BodyKind::Dynamic);
    assert_eq!(backend.body_translation(body), Some(Vec2::ZERO));
}
