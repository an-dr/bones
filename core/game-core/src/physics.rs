//! Bundles the rapier2d pipeline state `GameCore` steps each tick
//! (ADR-019: physics/collision is bought, not hand-rolled).

use std::num::NonZeroUsize;

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

pub struct Physics {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
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
}

impl Physics {
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
        }
    }

    /// Advances the simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
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
    /// — `Stopped` events are ignored (`game-core` only surfaces new
    /// contact, not separation, for now). Only fires for collider pairs
    /// where at least one collider requested `ActiveEvents::COLLISION_EVENTS`
    /// (set on every `EntityOp::Spawn`-created collider, not tilemap ones).
    pub fn drain_collision_starts(&mut self) -> Vec<(ColliderHandle, ColliderHandle)> {
        self.collision_events
            .try_iter()
            .filter_map(|event| match event {
                CollisionEvent::Started(a, b, _) => Some((a, b)),
                CollisionEvent::Stopped(..) => None,
            })
            .collect()
    }

    /// Whether two colliders are actually touching right now, not merely
    /// within rapier2d's speculative-contact prediction margin (the
    /// mechanism behind `CollisionEvent::Started` firing for colliders
    /// that are close but not yet overlapping). `drain_collision_starts`
    /// alone isn't enough to tell "real touch" apart from "about to
    /// touch".
    ///
    /// Deliberately checks each manifold point's `dist <= 0.0` rather than
    /// `ContactPair::has_any_active_contact`: that flag only tracks whether
    /// the *solver* needs to push the pair apart, which rapier2d skips
    /// entirely for a body-kind combination with no dynamic side (e.g. two
    /// tilemap colliders) — it would silently read `false` for a real,
    /// deep overlap in that case. Manifolds and their point distances are
    /// populated regardless of body kind, so this is the actual geometric
    /// ground truth.
    pub fn has_real_contact(&self, a: ColliderHandle, b: ColliderHandle) -> bool {
        self.narrow_phase.contact_pair(a, b).is_some_and(|pair| {
            pair.manifolds
                .iter()
                .any(|manifold| manifold.points.iter().any(|point| point.dist <= 0.0))
        })
    }

    /// The deepest current penetration between two colliders, in world
    /// units (a manifold point's negative `dist`, negated so a larger
    /// number means deeper overlap) — `0.0` if they aren't penetrating (or
    /// aren't in contact at all). Used to measure how much interpenetration
    /// the solver leaves under sustained pressure, not exposed for
    /// gameplay use (see `has_real_contact` for a touch/no-touch check).
    #[cfg(test)]
    pub(crate) fn penetration_depth(&self, a: ColliderHandle, b: ColliderHandle) -> f32 {
        self.narrow_phase
            .contact_pair(a, b)
            .into_iter()
            .flat_map(|pair| pair.manifolds.iter())
            .flat_map(|manifold| manifold.points.iter())
            .map(|point| (-point.dist).max(0.0))
            .fold(0.0, f32::max)
    }

    /// Every real (see `has_real_contact`) contact normal touching
    /// `collider` right now, each pointing *away from* `collider` (out of
    /// the obstacle, into free space) — the direction a commanded velocity
    /// must not have a component along, or it re-drives the body into
    /// whatever it's already touching. A collider with no contacts, or
    /// only speculative-margin ones, yields nothing.
    pub fn contact_normals(&self, collider: ColliderHandle) -> Vec<Vector<Real>> {
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
            .collect()
    }

    pub fn body_translation(&self, handle: RigidBodyHandle) -> Option<Vector<Real>> {
        self.bodies.get(handle).map(|body| *body.translation())
    }

    /// Removes a body and every collider attached to it. A no-op if
    /// `handle` names no body (already removed, or never valid).
    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
