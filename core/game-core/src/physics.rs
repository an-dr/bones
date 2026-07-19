//! Bundles the rapier2d pipeline state `GameCore` steps each tick
//! (ADR-019: physics/collision is bought, not hand-rolled).

use rapier2d::crossbeam::channel::{unbounded, Receiver};
use rapier2d::pipeline::ChannelEventCollector;
use rapier2d::prelude::*;

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
        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            gravity: vector![0.0, 0.0],
            integration_parameters: IntegrationParameters::default(),
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
