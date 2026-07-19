//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `rapier2d`, `glam`, `tiled`; see ADR-019's crate-sourcing
//! rationale). Renders by turning simulated state into `gfx/*` draw-command
//! batches, same as any other module — no rendering authority of its own.

use bones_messages::game_core::{Command, LoadTilemap, SpawnEntity};
use bones_messages::gfx::{DrawSprite, SetCamera};
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Bus, Envelope, Handler, Module, ModuleContext};
use rapier2d::prelude::{nalgebra, vector, ColliderBuilder, RigidBodyBuilder};

use crate::{load_collision_rects, Collider, Physics, SpriteAnimation, Transform};

/// The `gfx/*` layer game-core draws its entities on. Fixed rather than
/// configurable: this module owns exactly one concern (the game world),
/// same simplification `renderer`'s single global camera already makes.
const ENTITY_LAYER: u8 = 0;

pub struct GameCore {
    world: hecs::World,
    physics: Physics,
    bus: Option<Bus>,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            physics: Physics::new(),
            bus: None,
        }
    }

    fn publish<M: EncodeMessage>(&self, message: M) {
        let Some(bus) = &self.bus else {
            return;
        };
        bus.publish(Envelope {
            topic: M::TOPIC.to_string(),
            sender: "game-core".to_string(),
            correlation: None,
            payload: message.encode(),
        });
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

    /// Ignores an unparseable map rather than failing the module — a
    /// malformed asset from a WASM extension is that extension's mistake,
    /// not a reason to take down the whole simulation.
    fn load_tilemap(&mut self, load: LoadTilemap) {
        let Ok(rects) = load_collision_rects(load.tmx_bytes) else {
            return;
        };
        for rect in rects {
            let body = self
                .physics
                .bodies
                .insert(RigidBodyBuilder::fixed().translation(vector![rect.x, rect.y]));
            self.physics.colliders.insert_with_parent(
                ColliderBuilder::cuboid(rect.half_w, rect.half_h),
                body,
                &mut self.physics.bodies,
            );
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

        self.publish_gfx();
    }

    /// Turns every entity's simulated state into a `gfx::DrawSprite`, plus
    /// one fixed `gfx::SetCamera` — game-core has no rendering authority of
    /// its own (ADR-019), it only ever emits `gfx/*` the same as any
    /// extension would.
    ///
    /// Named `publish_gfx`, not `render`: `Module::render(&mut self)` is a
    /// distinct frame-phase hook (design/modules.md) this module doesn't
    /// use — game-core draws synchronously from `tick`'s `core/tick`
    /// dispatch (ADR-004), the same as `gfx::Command` handling elsewhere,
    /// not from the `render` phase. A same-named inherent method here
    /// previously shadowed by call-site ambiguity with the trait default;
    /// this name avoids the collision outright rather than relying on
    /// resolution rules.
    fn publish_gfx(&self) {
        self.publish(SetCamera {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        });
        for (_, (transform, animation)) in
            self.world.query::<(&Transform, &SpriteAnimation)>().iter()
        {
            self.publish(DrawSprite {
                id: animation.sprite_id,
                dst_x: transform.x as i32,
                dst_y: transform.y as i32,
                dst_w: animation.frame_w,
                dst_h: animation.frame_h,
                src_x: animation.current_src_x(),
                src_y: 0,
                src_w: animation.frame_w,
                src_h: animation.frame_h,
                layer: ENTITY_LAYER,
                angle: 0.0,
                flip_h: false,
                flip_v: false,
                tint: (255, 255, 255, 255),
            });
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
        match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(Command::SpawnEntity(spawn))) => self.spawn_entity(spawn),
            Ok(Some(Command::LoadTilemap(load))) => self.load_tilemap(load),
            _ => {}
        }
    }
}

impl Module for GameCore {
    fn name(&self) -> &str {
        "game-core"
    }

    /// Errors if no `Bus` service is available — a caller/embedder mistake
    /// (this module can't do anything useful without one), the same stance
    /// `renderer` takes for a missing `window-surface`.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("game-core/*");
        ctx.subscribe(Tick::TOPIC);
        self.bus = Some(
            ctx.consume_service::<Bus>()
                .ok_or("no Bus service available")?,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests;
