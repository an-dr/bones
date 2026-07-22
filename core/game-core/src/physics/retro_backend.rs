use std::collections::HashMap;

use glam::Vec2;

use super::{BodyHandle, BodyKind, ColliderHandle, PhysicsBackend, Shape};

struct Body {
    position: Vec2,
    velocity: Vec2,
    half_extents: Vec2,
    kind: BodyKind,
}

impl Body {
    /// `Fixed` and `Kinematic` bodies move exactly as commanded (or not at
    /// all) and are never displaced by a collision resolution pass — the
    /// same "platform/mover never gets pushed" contract every backend in
    /// this codebase gives those two kinds.
    fn is_pushable(&self) -> bool {
        matches!(self.kind, BodyKind::Dynamic | BodyKind::Frictionless)
    }
}

/// `PhysicsBackend` (ADR-021) with no mass, no impulses, and no solver: a
/// body moves by `velocity * dt` every step, then overlapping pushable
/// bodies are separated along the axis of least penetration (a minimum-
/// translation-vector push-apart, run once per step — not iterated to a
/// solved rest state the way rapier2d's contact solver is). This is the
/// "retro/arcade" feel ADR-021 asked for: immediate stop/start, no
/// momentum, no carried inertia.
#[derive(Default)]
pub struct RetroBackend {
    next_id: u64,
    bodies: HashMap<u64, Body>,
    collision_starts: Vec<(ColliderHandle, ColliderHandle)>,
    // Pairs already touching as of the last step — lets `step` report a
    // contact only on its first touching step, the same "start, not every
    // frame it's still touching" contract `drain_collision_starts` documents.
    touching: std::collections::HashSet<(u64, u64)>,
}

impl RetroBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn overlap(a: &Body, b: &Body) -> Option<Vec2> {
        let delta = b.position - a.position;
        let overlap_x = a.half_extents.x + b.half_extents.x - delta.x.abs();
        let overlap_y = a.half_extents.y + b.half_extents.y - delta.y.abs();
        (overlap_x > 0.0 && overlap_y > 0.0).then_some(Vec2::new(overlap_x, overlap_y))
    }

    /// Every current AABB overlap this step, as `(id_a, id_b, push_a,
    /// push_b)` — `push_a`/`push_b` are each body's share of the minimum-
    /// translation-vector separation, zero for a non-pushable body.
    fn overlaps(&self) -> Vec<(u64, u64, Vec2, Vec2)> {
        let ids: Vec<u64> = self.bodies.keys().copied().collect();
        let mut result = Vec::new();
        for (i, &id_a) in ids.iter().enumerate() {
            for &id_b in &ids[i + 1..] {
                let a = &self.bodies[&id_a];
                let b = &self.bodies[&id_b];
                let Some(overlap) = Self::overlap(a, b) else {
                    continue;
                };
                // Separate along the axis of least penetration — the
                // standard AABB minimum-translation-vector choice, so a
                // shallow corner clip resolves sideways rather than
                // launching a body vertically through a near-miss.
                let push = if overlap.x < overlap.y {
                    Vec2::new(if b.position.x < a.position.x { -overlap.x } else { overlap.x }, 0.0)
                } else {
                    Vec2::new(0.0, if b.position.y < a.position.y { -overlap.y } else { overlap.y })
                };
                let (a_pushable, b_pushable) = (a.is_pushable(), b.is_pushable());
                let (push_a, push_b) = match (a_pushable, b_pushable) {
                    (true, true) => (-push * 0.5, push * 0.5),
                    (true, false) => (-push, Vec2::ZERO),
                    (false, true) => (Vec2::ZERO, push),
                    (false, false) => (Vec2::ZERO, Vec2::ZERO),
                };
                result.push((id_a, id_b, push_a, push_b));
            }
        }
        result
    }
}

impl PhysicsBackend for RetroBackend {
    /// This backend has no non-rectangular collision concept — every
    /// `Shape` (including `Triangle`) is approximated as its own
    /// `half_extents` AABB bounding box, the same as `Shape::Rect`. Callers
    /// that need a real triangle collider should register that entity in
    /// the rapier2d world instead (`PhysicsWorlds`).
    fn spawn_shaped_body(
        &mut self,
        position: Vec2,
        half_extents: Vec2,
        _shape: Shape,
        kind: BodyKind,
    ) -> (BodyHandle, ColliderHandle) {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.insert(
            id,
            Body {
                position,
                velocity: Vec2::ZERO,
                half_extents,
                kind,
            },
        );
        (BodyHandle(id), ColliderHandle(id))
    }

    fn remove_body(&mut self, body: BodyHandle) {
        self.bodies.remove(&body.0);
        self.touching.retain(|&(a, b)| a != body.0 && b != body.0);
    }

    fn body_count(&self) -> usize {
        self.bodies.len()
    }

    fn set_velocity(&mut self, body: BodyHandle, velocity: Vec2) {
        if let Some(body) = self.bodies.get_mut(&body.0) {
            body.velocity = velocity;
        }
    }

    fn velocity(&self, body: BodyHandle) -> Option<Vec2> {
        self.bodies.get(&body.0).map(|body| body.velocity)
    }

    fn body_translation(&self, body: BodyHandle) -> Option<Vec2> {
        self.bodies.get(&body.0).map(|body| body.position)
    }

    fn set_body_state(&mut self, body: BodyHandle, position: Vec2, velocity: Vec2) {
        if let Some(body) = self.bodies.get_mut(&body.0) {
            body.position = position;
            body.velocity = velocity;
        }
    }

    fn is_kinematic(&self, body: BodyHandle) -> bool {
        self.bodies
            .get(&body.0)
            .is_some_and(|body| body.kind == BodyKind::Kinematic)
    }

    /// Integrates every non-`Fixed` body's position by `velocity * dt`,
    /// then separates overlapping pushable bodies once along the minimum-
    /// translation-vector axis — no iteration to a solved rest state, so a
    /// body driven continuously into an obstacle can still show a shallow
    /// per-step overlap that the next step's push resolves again, rather
    /// than converging within one step the way a real solver would.
    fn step(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            // `Fixed` never moves, full stop — even if something called
            // `set_velocity` on it (a caller mistake, not a case to
            // integrate anyway and rely on separation to undo).
            if body.kind != BodyKind::Fixed {
                body.position += body.velocity * dt;
            }
        }

        let overlaps = self.overlaps();
        let mut new_touching = std::collections::HashSet::new();
        for (id_a, id_b, push_a, push_b) in overlaps {
            if let Some(body) = self.bodies.get_mut(&id_a) {
                body.position += push_a;
            }
            if let Some(body) = self.bodies.get_mut(&id_b) {
                body.position += push_b;
            }
            let pair = (id_a.min(id_b), id_a.max(id_b));
            new_touching.insert(pair);
            if !self.touching.contains(&pair) {
                self.collision_starts
                    .push((ColliderHandle(id_a), ColliderHandle(id_b)));
            }
        }
        self.touching = new_touching;
    }

    fn drain_collision_starts(&mut self) -> Vec<(ColliderHandle, ColliderHandle)> {
        std::mem::take(&mut self.collision_starts)
    }

    /// Whether `a`/`b` were overlapping as of the most recent `step` —
    /// this backend has no speculative-contact margin (unlike rapier2d),
    /// so every pair `drain_collision_starts` reports already was a real,
    /// geometric overlap. Deliberately reads the `touching` set computed
    /// during `step`, not a fresh overlap check against the *current*
    /// (post-separation) positions: `step` immediately and exactly
    /// separates overlapping pushable bodies in the same call, so by the
    /// time a caller asks, the bodies are typically no longer overlapping
    /// even though the contact that just started was completely real.
    fn has_real_contact(&self, a: ColliderHandle, b: ColliderHandle) -> bool {
        self.touching.contains(&(a.0.min(b.0), a.0.max(b.0)))
    }

    /// This backend resolves contacts by directly moving positions apart
    /// during `step` rather than leaving a normal for the caller to react
    /// to afterward (there's no solver step to clamp a commanded velocity
    /// against) — always empty. `game-core`'s contact-clamping logic is a
    /// no-op against a body with no normals, which is the correct outcome:
    /// this backend already fully resolved the overlap for this step.
    fn contact_normals(&self, _collider: ColliderHandle) -> Vec<Vec2> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
