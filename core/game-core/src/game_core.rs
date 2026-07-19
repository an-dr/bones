//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `rapier2d`, `glam`, `tiled`; see ADR-019's crate-sourcing
//! rationale). Renders by turning simulated state into `gfx/*` draw-command
//! batches, same as any other module — no rendering authority of its own.

use bones_messages::game_core::{Command, SpawnEntity};
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, Message};
use bus::{Envelope, Handler, Module, ModuleContext};
use rapier2d::prelude::{nalgebra, vector, ColliderBuilder, RigidBodyBuilder};

use crate::{Collider, Physics, SpriteAnimation, Transform};

pub struct GameCore {
    world: hecs::World,
    physics: Physics,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            physics: Physics::new(),
        }
    }

    fn spawn_entity(&mut self, spawn: SpawnEntity) {
        let transform = Transform {
            x: spawn.x,
            y: spawn.y,
        };
        let animation = SpriteAnimation::new(
            spawn.sprite_id,
            spawn.frame_w,
            spawn.frame_h,
            spawn.frame_count,
            spawn.frame_duration,
        );

        if spawn.collider_half_w > 0.0 && spawn.collider_half_h > 0.0 {
            let body = self
                .physics
                .bodies
                .insert(RigidBodyBuilder::dynamic().translation(vector![spawn.x, spawn.y]));
            let collider = self.physics.colliders.insert_with_parent(
                ColliderBuilder::cuboid(spawn.collider_half_w, spawn.collider_half_h),
                body,
                &mut self.physics.bodies,
            );
            self.world
                .spawn((transform, animation, Collider { body, collider }));
        } else {
            self.world.spawn((transform, animation));
        }
    }

    fn tick(&mut self, dt: f32) {
        for (_, animation) in self.world.query_mut::<&mut SpriteAnimation>() {
            animation.advance(dt);
        }

        self.physics.step(dt);

        for (_, (transform, collider)) in self.world.query_mut::<(&mut Transform, &Collider)>() {
            if let Some(translation) = self.physics.body_translation(collider.body) {
                transform.x = translation.x;
                transform.y = translation.y;
            }
        }
    }
}

impl Default for GameCore {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for GameCore {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic == Tick::TOPIC {
            if let Ok(tick) = Tick::decode(&envelope.payload) {
                self.tick(tick.dt);
            }
            return;
        }
        if let Ok(Some(Command::SpawnEntity(spawn))) =
            Command::decode(&envelope.topic, &envelope.payload)
        {
            self.spawn_entity(spawn);
        }
    }
}

impl Module for GameCore {
    fn name(&self) -> &str {
        "game-core"
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("game-core/*");
        ctx.subscribe(Tick::TOPIC);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
