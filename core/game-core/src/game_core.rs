//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `glam`, `tiled`; physics comes from `physics::PhysicsBackend`,
//! ADR-021). Renders by turning simulated state into `gfx/*` draw-command
//! batches, same as any other module — no rendering authority of its own.

use std::collections::HashMap;

use bones_messages::game_core::{
    BodyKind as WireBodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap,
};
use bones_messages::gfx::{Clear, DrawRect, DrawSprite, SetCamera};
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Bus, Envelope, Handler, Module, ModuleContext};
use glam::Vec2;
use physics::{BodyHandle, ColliderHandle, PhysicsBackend};
use physics_rapier2d::Rapier2dBackend;

use crate::{load_collision_rects, Collider, SpriteAnimation, SquareColor, Transform};

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

/// Below this squared linear speed, an entity counts as "at rest" for
/// sprite-animation gating — guards against floating-point residue (a
/// stopped body's velocity settles near, not exactly, zero) rather than
/// requiring an exact `0.0`.
const MOVING_SPEED_THRESHOLD_SQUARED: f32 = 0.01;

/// Color for `EntityOp::SetDebugHitboxes`'s outline — distinct from every
/// other color this module draws (sprite tint, obstacle/tilemap
/// `square_color`), so it reads unambiguously as a debug overlay.
const DEBUG_HITBOX_COLOR: (u8, u8, u8, u8) = (255, 255, 0, 255);

pub struct GameCore {
    world: hecs::World,
    physics: Rapier2dBackend,
    bus: Option<Bus>,
    // Maps the caller's own `EntityOp`-carried `entity_id` to the
    // `hecs::Entity` it became — the caller's addressing scheme never sees
    // a raw `hecs::Entity`, which is this module's own internal handle.
    entities: HashMap<u32, hecs::Entity>,
    // The inverse of `entities`, restricted to `EntityOp::Spawn`-created
    // colliders (tilemap colliders never appear here) — lets a
    // `PhysicsBackend` collision event's `ColliderHandle` pair translate
    // back into the caller-assigned `entity_id` pair a `game-core/collision`
    // event names.
    entity_ids_by_collider: HashMap<ColliderHandle, u32>,
    // Global toggle for `EntityOp::SetDebugHitboxes` — not per-entity, so
    // it lives here rather than as a component.
    debug_hitboxes: bool,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            physics: Rapier2dBackend::new(),
            bus: None,
            entities: HashMap::new(),
            entity_ids_by_collider: HashMap::new(),
            debug_hitboxes: false,
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
                body_kind,
            } => self.spawn_entity(
                entity_id,
                x,
                y,
                sprite,
                square_color,
                collider_half_w,
                collider_half_h,
                body_kind,
            ),
            EntityOp::SetVelocity { entity_id, vx, vy } => self.set_velocity(entity_id, vx, vy),
            EntityOp::Despawn { entity_id } => self.despawn(entity_id),
            EntityOp::SetColor { entity_id, color } => self.set_color(entity_id, color),
            EntityOp::SetDebugHitboxes { enabled } => self.debug_hitboxes = enabled,
        }
    }

    /// A no-op if `entity_id` names no entity, or one with no
    /// `SquareColor` — a sprite entity has none to overwrite.
    fn set_color(&mut self, entity_id: u32, color: (u8, u8, u8, u8)) {
        let Some(&entity) = self.entities.get(&entity_id) else {
            return;
        };
        if let Ok(mut square_color) = self.world.get::<&mut SquareColor>(entity) {
            square_color.0 = color;
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
        body_kind: WireBodyKind,
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
            let kind = match body_kind {
                WireBodyKind::Dynamic => physics::BodyKind::Dynamic,
                WireBodyKind::Kinematic => physics::BodyKind::Kinematic,
                WireBodyKind::Frictionless => physics::BodyKind::Frictionless,
            };
            let (body, collider) = self.physics.spawn_body(
                Vec2::new(x, y),
                Vec2::new(collider_half_w, collider_half_h),
                kind,
            );
            builder.add(Collider {
                body,
                collider,
                half_w: collider_half_w,
                half_h: collider_half_h,
            });
            self.entity_ids_by_collider.insert(collider, entity_id);
        }

        let entity = self.world.spawn(builder.build());
        self.entities.insert(entity_id, entity);
    }

    /// A no-op if `entity_id` names no entity, or one with no collider —
    /// a purely visual entity has no physics body to set velocity on.
    fn set_velocity(&mut self, entity_id: u32, vx: f32, vy: f32) {
        let Some(&entity) = self.entities.get(&entity_id) else {
            return;
        };
        let Ok(collider) = self.world.get::<&Collider>(entity) else {
            return;
        };
        self.physics.set_velocity(collider.body, Vec2::new(vx, vy));
    }

    /// A no-op if `entity_id` names no entity — despawning twice, or an id
    /// never spawned, is not an error.
    fn despawn(&mut self, entity_id: u32) {
        let Some(entity) = self.entities.remove(&entity_id) else {
            return;
        };
        if let Ok(collider) = self.world.get::<&Collider>(entity) {
            self.entity_ids_by_collider.remove(&collider.collider);
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
            let (body, collider) = self.physics.spawn_body(
                Vec2::new(rect.x, rect.y),
                Vec2::new(rect.half_w, rect.half_h),
                physics::BodyKind::Fixed,
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
        self.physics.step(dt);
        self.clamp_velocities_against_contacts();

        // Animation only advances while actually moving — an entity with
        // no collider (nothing driving it) or one at rest freezes on its
        // current frame instead of animating in place.
        for (_, (animation, collider)) in self
            .world
            .query_mut::<(&mut SpriteAnimation, Option<&Collider>)>()
        {
            let moving = collider
                .and_then(|collider| self.physics.velocity(collider.body))
                .is_some_and(|velocity| {
                    velocity.length_squared() > MOVING_SPEED_THRESHOLD_SQUARED
                });
            if moving {
                animation.advance(dt);
            }
        }

        for (_, (transform, collider)) in self.world.query_mut::<(&mut Transform, &Collider)>() {
            if let Some(translation) = self.physics.body_translation(collider.body) {
                transform.x = translation.x;
                transform.y = translation.y;
            }
        }

        self.publish_collisions();
        self.publish_gfx();
    }

    /// Stops a body's commanded velocity from re-driving it into whatever
    /// it's already touching — the actual mechanism behind visibly
    /// overlapping squares under sustained held input. `EntityOp::SetVelocity`
    /// hard-overwrites a body's linear velocity every tick from held input,
    /// which fights rapier2d's compliant contact solver: the solver's
    /// per-step corrective push competes against the same inward velocity
    /// being re-applied next tick, and can lose. Zeroing the
    /// contact-normal-aligned component of velocity after each step (but
    /// only the inward part, so motion along the surface — sliding along a
    /// wall — is untouched) keeps a held direction from winning that fight.
    ///
    /// `Kinematic` bodies are excluded: they move exactly as commanded by
    /// design (the "platform/mover" contract) and are never pushed, so
    /// there's nothing to clamp.
    fn clamp_velocities_against_contacts(&mut self) {
        let clamped: Vec<(BodyHandle, Vec2)> = self
            .world
            .query::<&Collider>()
            .iter()
            .filter_map(|(_, collider)| {
                if self.physics.is_kinematic(collider.body) {
                    return None;
                }
                let velocity = self.physics.velocity(collider.body)?;
                if velocity == Vec2::ZERO {
                    return None;
                }
                let mut clamped_velocity = velocity;
                for normal in self.physics.contact_normals(collider.collider) {
                    let inward = clamped_velocity.dot(normal);
                    if inward > 0.0 {
                        clamped_velocity -= normal * inward;
                    }
                }
                (clamped_velocity != velocity).then_some((collider.body, clamped_velocity))
            })
            .collect();
        for (body, velocity) in clamped {
            self.physics.set_velocity(body, velocity);
        }
    }

    /// Translates every rapier2d contact-start this tick's `step` produced
    /// into a `game-core/collision` event, by `ColliderHandle`. A pair
    /// where either side isn't in `entity_ids_by_collider` (a tilemap
    /// collider, or a collider whose entity was already despawned this
    /// tick) is silently skipped rather than published with a missing id.
    /// `CollisionEvent::Started` alone isn't proof of a real touch — rapier2d
    /// uses speculative contacts, so a pair can appear "Started" while still
    /// a hair's width apart (the mechanism behind real-world phantom "hit"
    /// reports with no visible contact). `has_real_contact` filters those
    /// out by checking the actual contact manifold, not just the event.
    fn publish_collisions(&mut self) {
        for (collider_a, collider_b) in self.physics.drain_collision_starts() {
            if !self.physics.has_real_contact(collider_a, collider_b) {
                continue;
            }
            let entity_id_a = self.entity_ids_by_collider.get(&collider_a).copied();
            let entity_id_b = self.entity_ids_by_collider.get(&collider_b).copied();
            if let (Some(entity_id_a), Some(entity_id_b)) = (entity_id_a, entity_id_b) {
                self.publish(Collision {
                    entity_id_a,
                    entity_id_b,
                });
            }
        }
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
        if self.debug_hitboxes {
            // Every collider-bearing entity, sprite or square alike — a
            // sprite's drawn frame and a square's fill can each differ
            // slightly from what actually collides, which is exactly what
            // this overlay is for checking. Drawn on `ENTITY_LAYER` after
            // every normal draw above, so the outline lands on top.
            for (_, (transform, collider)) in self.world.query::<(&Transform, &Collider)>().iter() {
                self.publish(DrawRect {
                    x: (transform.x - collider.half_w) as i32,
                    y: (transform.y - collider.half_h) as i32,
                    w: (collider.half_w * 2.0) as u32,
                    h: (collider.half_h * 2.0) as u32,
                    filled: false,
                    color: DEBUG_HITBOX_COLOR,
                    layer: ENTITY_LAYER,
                });
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
