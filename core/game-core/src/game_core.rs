//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `rapier2d`, `glam`, `tiled`; see ADR-019's crate-sourcing
//! rationale). Renders by turning simulated state into `gfx/*` draw-command
//! batches, same as any other module — no rendering authority of its own.

use std::collections::HashMap;

use bones_messages::game_core::{EntityOp, EntityOpMessage, LoadTilemap};
use bones_messages::gfx::{Clear, DrawRect, DrawSprite, SetCamera};
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Bus, Envelope, Handler, Module, ModuleContext};
use rapier2d::prelude::{nalgebra, vector, ColliderBuilder, RigidBodyBuilder};

use crate::{load_collision_rects, Collider, Physics, SpriteAnimation, SquareColor, Transform};

/// The `gfx/*` layer game-core draws its entities on. Fixed rather than
/// configurable: this module owns exactly one concern (the game world),
/// same simplification `renderer`'s single global camera already makes.
const ENTITY_LAYER: u8 = 0;

/// Fixed color for every collider `load_tilemap` creates — visually
/// distinct from `EntityOp::Spawn`'s caller-chosen `square_color`, so a
/// tilemap wall reads differently from a spawned obstacle. Discovered as a
/// real need, not speculative: an invisible tilemap collider reads as a
/// bug ("invisible wall") rather than an intentional obstacle.
const TILEMAP_COLLIDER_COLOR: (u8, u8, u8, u8) = (90, 90, 100, 255);

pub struct GameCore {
    world: hecs::World,
    physics: Physics,
    bus: Option<Bus>,
    // Maps the caller's own `EntityOp`-carried `entity_id` to the
    // `hecs::Entity` it became — the caller's addressing scheme never sees
    // a raw `hecs::Entity`, which is this module's own internal handle.
    entities: HashMap<u32, hecs::Entity>,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            physics: Physics::new(),
            bus: None,
            entities: HashMap::new(),
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

    fn apply_entity_op(&mut self, op: EntityOp) {
        match op {
            EntityOp::Spawn {
                entity_id,
                x,
                y,
                sprite,
                square_color,
                collider_half_w,
                collider_half_h,
            } => self.spawn_entity(
                entity_id,
                x,
                y,
                sprite,
                square_color,
                collider_half_w,
                collider_half_h,
            ),
            EntityOp::SetVelocity { entity_id, vx, vy } => self.set_velocity(entity_id, vx, vy),
            EntityOp::Despawn { entity_id } => self.despawn(entity_id),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_entity(
        &mut self,
        entity_id: u32,
        x: f32,
        y: f32,
        sprite: Option<bones_messages::game_core::Sprite>,
        square_color: (u8, u8, u8, u8),
        collider_half_w: f32,
        collider_half_h: f32,
    ) {
        // Replaces any entity already spawned under this id — the same
        // replace-on-republish semantics `gfx::DrawSprite` batches use.
        self.despawn(entity_id);

        let transform = Transform { x, y };
        let animation = sprite.map(|sprite| {
            SpriteAnimation::new(
                sprite.sprite_id,
                sprite.frame_w,
                sprite.frame_h,
                sprite.frame_count,
                sprite.frame_duration,
            )
        });

        let mut builder = hecs::EntityBuilder::new();
        builder.add(transform);
        match animation {
            Some(animation) => {
                builder.add(animation);
            }
            None => {
                builder.add(SquareColor(square_color));
            }
        }

        if collider_half_w > 0.0 && collider_half_h > 0.0 {
            let body = self
                .physics
                .bodies
                .insert(RigidBodyBuilder::dynamic().translation(vector![x, y]));
            let collider = self.physics.colliders.insert_with_parent(
                ColliderBuilder::cuboid(collider_half_w, collider_half_h),
                body,
                &mut self.physics.bodies,
            );
            builder.add(Collider {
                body,
                collider,
                half_w: collider_half_w,
                half_h: collider_half_h,
            });
        }

        let entity = self.world.spawn(builder.build());
        self.entities.insert(entity_id, entity);
    }

    /// A no-op if `entity_id` names no entity, or one with no collider —
    /// a purely visual entity has no rapier2d body to set velocity on.
    fn set_velocity(&mut self, entity_id: u32, vx: f32, vy: f32) {
        let Some(&entity) = self.entities.get(&entity_id) else {
            return;
        };
        let Ok(collider) = self.world.get::<&Collider>(entity) else {
            return;
        };
        if let Some(body) = self.physics.bodies.get_mut(collider.body) {
            body.set_linvel(vector![vx, vy], true);
        }
    }

    /// A no-op if `entity_id` names no entity — despawning twice, or an id
    /// never spawned, is not an error.
    fn despawn(&mut self, entity_id: u32) {
        let Some(entity) = self.entities.remove(&entity_id) else {
            return;
        };
        if let Ok(collider) = self.world.get::<&Collider>(entity) {
            self.physics.remove_body(collider.body);
        }
        let _ = self.world.despawn(entity);
    }

    /// Ignores an unparseable map rather than failing the module — a
    /// malformed asset from a WASM extension is that extension's mistake,
    /// not a reason to take down the whole simulation. Each collision rect
    /// becomes an ordinary square entity (`Transform` + `SquareColor` +
    /// `Collider`, fixed rather than dynamic) — not a raw physics body with
    /// no ECS presence — so `publish_gfx`'s existing square-drawing query
    /// renders it for free instead of it being an invisible wall.
    fn load_tilemap(&mut self, load: LoadTilemap) {
        let Ok(rects) = load_collision_rects(load.tmx_bytes) else {
            return;
        };
        for rect in rects {
            let body = self
                .physics
                .bodies
                .insert(RigidBodyBuilder::fixed().translation(vector![rect.x, rect.y]));
            let collider = self.physics.colliders.insert_with_parent(
                ColliderBuilder::cuboid(rect.half_w, rect.half_h),
                body,
                &mut self.physics.bodies,
            );
            self.world.spawn((
                Transform {
                    x: rect.x,
                    y: rect.y,
                },
                SquareColor(TILEMAP_COLLIDER_COLOR),
                Collider {
                    body,
                    collider,
                    half_w: rect.half_w,
                    half_h: rect.half_h,
                },
            ));
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

    /// Turns every entity's simulated state into a `gfx::DrawSprite` or
    /// `gfx::DrawRect`, plus one `gfx::Clear` and a fixed `gfx::SetCamera`
    /// — game-core has no rendering authority of its own (ADR-019), it
    /// only ever emits `gfx/*` the same as any extension would.
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
        // Without this, every previous frame's pixels stay on screen under
        // this frame's draws (the renderer only clears when told to) —
        // discovered as a real visual bug (streaking/smearing), not a
        // hypothetical.
        self.publish(Clear {
            r: 20,
            g: 20,
            b: 20,
            a: 255,
        });
        self.publish(SetCamera {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        });
        for (_, (transform, animation)) in
            self.world.query::<(&Transform, &SpriteAnimation)>().iter()
        {
            // `Transform` is the entity's center (matching rapier2d's own
            // convention); `DrawSprite::dst_x`/`dst_y` is a top-left corner
            // — without this conversion the visible sprite sits offset from
            // its actual collider, so contact with other colliders looks
            // wrong even though the physics underneath is already correct.
            self.publish(DrawSprite {
                id: animation.sprite_id,
                dst_x: (transform.x - animation.frame_w as f32 / 2.0) as i32,
                dst_y: (transform.y - animation.frame_h as f32 / 2.0) as i32,
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
        for (_, (transform, color, collider)) in self
            .world
            .query::<(&Transform, &SquareColor, &Collider)>()
            .iter()
        {
            self.publish(DrawRect {
                x: (transform.x - collider.half_w) as i32,
                y: (transform.y - collider.half_h) as i32,
                w: (collider.half_w * 2.0) as u32,
                h: (collider.half_h * 2.0) as u32,
                filled: true,
                color: color.0,
                layer: ENTITY_LAYER,
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
        if envelope.topic == EntityOpMessage::TOPIC {
            if let Ok(EntityOpMessage(op)) = EntityOpMessage::decode(&envelope.payload) {
                self.apply_entity_op(op);
            }
            return;
        }
        if envelope.topic == LoadTilemap::TOPIC {
            if let Ok(load) = LoadTilemap::decode(&envelope.payload) {
                self.load_tilemap(load);
            }
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
