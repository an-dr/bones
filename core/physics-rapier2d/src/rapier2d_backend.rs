use std::collections::HashMap;
use std::num::NonZeroUsize;

use glam::Vec2;
use physics::{BodyHandle, BodyKind, ColliderHandle, PhysicsBackend};
use rapier2d::crossbeam::channel::{unbounded, Receiver};
use rapier2d::pipeline::ChannelEventCollector;
use rapier2d::prelude::*;

/// rapier2d 0.22's default is 30.0. Higher pushes overlapping bodies apart
/// faster/harder per step — tuned against visible interpenetration under
/// sustained driving force, not derived from a formula.
const CONTACT_NATURAL_FREQUENCY: f32 = 120.0;

/// rapier2d 0.22's default is 5.0 (a fairly soft/compliant spring). Lower
/// is stiffer; paired with the higher frequency above without going low
/// enough to reintroduce bounce/jitter.
const CONTACT_DAMPING_RATIO: f32 = 2.0;

/// rapier2d 0.22's default is 4. More iterations converge better at the
/// higher stiffness above instead of overshooting.
const NUM_SOLVER_ITERATIONS: usize = 8;

/// rapier2d 0.22's default is 1.
const NUM_INTERNAL_PGS_ITERATIONS: usize = 2;

/// Linear damping for `BodyKind::Frictionless` bodies: high enough that
/// velocity decays to rest within a fraction of a tick once nothing is
/// pushing the body, so it never visibly coasts or drifts after contact
/// ends.
const FRICTIONLESS_LINEAR_DAMPING: f32 = 50.0;

/// `PhysicsBackend` (ADR-021) backed by a rapier2d pipeline — the same
/// simulation and tuning `game-core` used directly before the split, now
/// behind the backend-agnostic trait. Own opaque `u64` handles are minted
/// per spawn and mapped to rapier2d's own handles internally, so this
/// crate never leaks `RigidBodyHandle`/`ColliderHandle` to a caller.
pub struct Rapier2dBackend {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    collision_events: Receiver<CollisionEvent>,
    event_handler: ChannelEventCollector,
    next_id: u64,
    body_handles: HashMap<u64, RigidBodyHandle>,
    collider_handles: HashMap<u64, ColliderHandle_>,
    body_ids: HashMap<RigidBodyHandle, u64>,
    collider_ids: HashMap<ColliderHandle_, u64>,
}

// rapier2d's own `ColliderHandle` shadows `physics::ColliderHandle` (both
// imported); this alias disambiguates the rapier2d side everywhere it's
// used as a map key/value below.
type ColliderHandle_ = rapier2d::geometry::ColliderHandle;

impl Rapier2dBackend {
    /// No gravity by default — a 2D top-down game (the working example is
    /// an RPG, ADR-019) has no natural "down"; a side-view game can set it
    /// itself via a later API if this module grows one.
    pub fn new() -> Self {
        let (collision_sender, collision_events) = unbounded();
        let (force_sender, _force_events) = unbounded();
        let mut integration_parameters = IntegrationParameters::default();
        // Stiffer contacts than rapier2d's defaults (30.0 / 5.0): the
        // default spring-based contact resolution is compliant enough that
        // a body driven continuously into an obstacle (this module's usual
        // case — a player entity holding a direction key against a wall)
        // visibly interpenetrates rather than fully separating each step.
        // Raising the natural frequency and lowering the damping ratio
        // makes the contact push back harder and settle faster; more
        // solver/stabilization iterations improve convergence at that
        // higher stiffness instead of overshooting/jittering.
        integration_parameters.contact_natural_frequency = CONTACT_NATURAL_FREQUENCY;
        integration_parameters.contact_damping_ratio = CONTACT_DAMPING_RATIO;
        integration_parameters.num_solver_iterations =
            NonZeroUsize::new(NUM_SOLVER_ITERATIONS).expect("NUM_SOLVER_ITERATIONS is nonzero");
        integration_parameters.num_internal_pgs_iterations = NUM_INTERNAL_PGS_ITERATIONS;
        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            gravity: vector![0.0, 0.0],
            integration_parameters,
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            collision_events,
            event_handler: ChannelEventCollector::new(collision_sender, force_sender),
            next_id: 0,
            body_handles: HashMap::new(),
            collider_handles: HashMap::new(),
            body_ids: HashMap::new(),
            collider_ids: HashMap::new(),
        }
    }

    /// The deepest current penetration between two colliders, in world
    /// units (a manifold point's negative `dist`, negated so a larger
    /// number means deeper overlap) — `0.0` if they aren't penetrating (or
    /// aren't in contact at all). Used to measure how much interpenetration
    /// the solver leaves under sustained pressure, not exposed for
    /// gameplay use (see `has_real_contact` for a touch/no-touch check).
    #[cfg(test)]
    pub(crate) fn penetration_depth(&self, a: ColliderHandle, b: ColliderHandle) -> f32 {
        let (Some(&a), Some(&b)) = (self.collider_handles.get(&a.0), self.collider_handles.get(&b.0))
        else {
            return 0.0;
        };
        self.narrow_phase
            .contact_pair(a, b)
            .into_iter()
            .flat_map(|pair| pair.manifolds.iter())
            .flat_map(|manifold| manifold.points.iter())
            .map(|point| (-point.dist).max(0.0))
            .fold(0.0, f32::max)
    }
}

impl Default for Rapier2dBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsBackend for Rapier2dBackend {
    fn spawn_body(
        &mut self,
        position: Vec2,
        half_extents: Vec2,
        kind: BodyKind,
    ) -> (BodyHandle, ColliderHandle) {
        let rigid_body = match kind {
            BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
            BodyKind::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
            BodyKind::Fixed => RigidBodyBuilder::fixed(),
            BodyKind::Frictionless => RigidBodyBuilder::dynamic()
                .linear_damping(FRICTIONLESS_LINEAR_DAMPING)
                .lock_rotations(),
        };
        let body = self
            .bodies
            .insert(rigid_body.translation(vector![position.x, position.y]));
        let collider = self.colliders.insert_with_parent(
            ColliderBuilder::cuboid(half_extents.x, half_extents.y)
                .active_events(ActiveEvents::COLLISION_EVENTS),
            body,
            &mut self.bodies,
        );

        let id = self.next_id;
        self.next_id += 1;
        self.body_handles.insert(id, body);
        self.collider_handles.insert(id, collider);
        self.body_ids.insert(body, id);
        self.collider_ids.insert(collider, id);

        (BodyHandle(id), ColliderHandle(id))
    }

    /// Removes a body and every collider attached to it. A no-op if
    /// `body` names no body (already removed, never valid, or valid only
    /// in a different `Rapier2dBackend` instance).
    fn remove_body(&mut self, body: BodyHandle) {
        let Some(handle) = self.body_handles.remove(&body.0) else {
            return;
        };
        self.body_ids.remove(&handle);
        if let Some(collider) = self.collider_handles.remove(&body.0) {
            self.collider_ids.remove(&collider);
        }
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    fn set_velocity(&mut self, body: BodyHandle, velocity: Vec2) {
        let Some(&handle) = self.body_handles.get(&body.0) else {
            return;
        };
        if let Some(body) = self.bodies.get_mut(handle) {
            body.set_linvel(vector![velocity.x, velocity.y], true);
        }
    }

    fn velocity(&self, body: BodyHandle) -> Option<Vec2> {
        let &handle = self.body_handles.get(&body.0)?;
        let body = self.bodies.get(handle)?;
        let v = body.linvel();
        Some(Vec2::new(v.x, v.y))
    }

    fn body_translation(&self, body: BodyHandle) -> Option<Vec2> {
        let &handle = self.body_handles.get(&body.0)?;
        let body = self.bodies.get(handle)?;
        let t = body.translation();
        Some(Vec2::new(t.x, t.y))
    }

    fn set_body_state(&mut self, body: BodyHandle, position: Vec2, velocity: Vec2) {
        let Some(&handle) = self.body_handles.get(&body.0) else {
            return;
        };
        if let Some(body) = self.bodies.get_mut(handle) {
            body.set_translation(vector![position.x, position.y], true);
            body.set_linvel(vector![velocity.x, velocity.y], true);
        }
    }

    fn is_kinematic(&self, body: BodyHandle) -> bool {
        let Some(&handle) = self.body_handles.get(&body.0) else {
            return false;
        };
        self.bodies
            .get(handle)
            .is_some_and(|body| body.is_kinematic())
    }

    /// Advances the simulation by `dt` seconds.
    fn step(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &(),
            &self.event_handler,
        );
    }

    /// Drains every `CollisionEvent::Started` produced since the last call
    /// — `Stopped` events are ignored (this backend only surfaces new
    /// contact, not separation, for now). Only fires for collider pairs
    /// where at least one collider requested `ActiveEvents::COLLISION_EVENTS`
    /// (set on every `spawn_body`-created collider).
    fn drain_collision_starts(&mut self) -> Vec<(ColliderHandle, ColliderHandle)> {
        self.collision_events
            .try_iter()
            .filter_map(|event| match event {
                CollisionEvent::Started(a, b, _) => Some((a, b)),
                CollisionEvent::Stopped(..) => None,
            })
            .filter_map(|(a, b)| {
                let a = *self.collider_ids.get(&a)?;
                let b = *self.collider_ids.get(&b)?;
                Some((ColliderHandle(a), ColliderHandle(b)))
            })
            .collect()
    }

    /// Whether two colliders are actually touching right now, not merely
    /// within rapier2d's speculative-contact prediction margin (the
    /// mechanism behind `CollisionEvent::Started` firing for colliders
    /// that are close but not yet overlapping).
    ///
    /// Deliberately checks each manifold point's `dist <= 0.0` rather than
    /// `ContactPair::has_any_active_contact`: that flag only tracks whether
    /// the *solver* needs to push the pair apart, which rapier2d skips
    /// entirely for a body-kind combination with no dynamic side (e.g. two
    /// tilemap colliders) — it would silently read `false` for a real,
    /// deep overlap in that case. Manifolds and their point distances are
    /// populated regardless of body kind, so this is the actual geometric
    /// ground truth.
    fn has_real_contact(&self, a: ColliderHandle, b: ColliderHandle) -> bool {
        let (Some(&a), Some(&b)) = (self.collider_handles.get(&a.0), self.collider_handles.get(&b.0))
        else {
            return false;
        };
        self.narrow_phase.contact_pair(a, b).is_some_and(|pair| {
            pair.manifolds
                .iter()
                .any(|manifold| manifold.points.iter().any(|point| point.dist <= 0.0))
        })
    }

    /// Every real (see `has_real_contact`) contact normal touching
    /// `collider` right now, each pointing *away from* `collider` (out of
    /// the obstacle, into free space) — the direction a commanded velocity
    /// must not have a component along, or it re-drives the body into
    /// whatever it's already touching.
    fn contact_normals(&self, collider: ColliderHandle) -> Vec<Vec2> {
        let Some(&collider) = self.collider_handles.get(&collider.0) else {
            return Vec::new();
        };
        self.narrow_phase
            .contact_pairs_with(collider)
            .filter(|pair| {
                pair.manifolds
                    .iter()
                    .any(|manifold| manifold.points.iter().any(|point| point.dist <= 0.0))
            })
            .flat_map(|pair| {
                // `ContactPair::collider1`/`collider2` name which side is
                // which; the manifold normal is defined pointing from
                // collider1 toward collider2 (confirmed against this
                // exact 0.22.0 build), so it must be flipped when the
                // queried collider is on the collider2 side.
                let flip = if pair.collider1 == collider {
                    1.0
                } else {
                    -1.0
                };
                pair.manifolds
                    .iter()
                    .map(move |manifold| manifold.data.normal * flip)
            })
            .map(|n| Vec2::new(n.x, n.y))
            .collect()
    }
}

#[cfg(test)]
mod tests;
