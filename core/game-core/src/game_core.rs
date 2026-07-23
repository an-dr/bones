//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `glam`, `tiled`; physics comes from this crate's own
//! `physics::PhysicsBackend`, ADR-021/ADR-022). Renders by turning
//! simulated state into `gfx/*` draw-command batches, same as any other
//! module — no rendering authority of its own.

use std::collections::HashMap;

use bones_messages::game_core::{
    BodyKind as WireBodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, PhysicsWorlds,
    Shape as WireShape,
};
use bones_messages::gfx::{Clear, DrawRect, DrawSprite, DrawTriangle, LoadSprite, SetCamera};
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Bus, Envelope, Handler, Module, ModuleContext};
use glam::Vec2;

use crate::graphics::{SpriteAnimation, SquareColor, Transform};
use crate::physics::{
    self, BodyHandle, Collider, ColliderHandle, PhysicsBackend, PhysicsWorldKind, Rapier2dBackend,
    RetroBackend, Shape, WorldBody,
};
use crate::tiles::{load_collision_rects, load_tile_draws, map_pixel_size, TileDraw};

/// The `gfx/*` layer game-core draws its entities on.
const ENTITY_LAYER: u8 = 1;

/// The `gfx/*` layer `load_tilemap`'s own `"Ground"` tile draws publish
/// on — deliberately `0`, one below `ENTITY_LAYER`, so the ground always
/// renders underneath every entity regardless of publish order (both
/// come from this same module, same tick, so publish order alone
/// wouldn't otherwise guarantee it). This is also the layer number a
/// caller with its *own* background content (no `"Ground"` layer, or a
/// caller not using `load_tilemap` at all) should use, for the same
/// reason.
const BACKGROUND_LAYER: u8 = 0;

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
    rapier2d: Rapier2dBackend,
    retro: RetroBackend,
    bus: Option<Bus>,
    // Maps the caller's own `EntityOp`-carried `entity_id` to the
    // `hecs::Entity` it became — the caller's addressing scheme never sees
    // a raw `hecs::Entity`, which is this module's own internal handle.
    entities: HashMap<u32, hecs::Entity>,
    // The inverse of `entities`, restricted to `EntityOp::Spawn`-created
    // colliders (tilemap colliders never appear here) — lets a
    // `PhysicsBackend` collision event's `(world, ColliderHandle)` pair
    // translate back into the caller-assigned `entity_id` pair a
    // `game-core/collision` event names. Keyed by world too: each backend
    // instance mints its own `ColliderHandle`s independently, so the same
    // numeric handle can exist in both worlds at once, naming two
    // different entities (or the same one, in the multi-world case).
    entity_ids_by_collider: HashMap<(PhysicsWorldKind, ColliderHandle), u32>,
    // Global toggle for `EntityOp::SetDebugHitboxes` — not per-entity, so
    // it lives here rather than as a component.
    debug_hitboxes: bool,
    // Global toggle for `EntityOp::SetPaused` — see `tick`'s early return.
    paused: bool,
    // Resolved once per `load_tilemap` call (not re-parsed every tick) and
    // replayed as `gfx::DrawSprite` every `publish_gfx` — the same
    // "publish every tick regardless" contract every other `gfx/*` output
    // here follows, just sourced from a one-time parse instead of live
    // simulation state, since a loaded tilemap's ground never changes.
    // Replaced wholesale by a later `load_tilemap` call, the same
    // replace-on-reload semantics collision rects already have.
    tile_draws: Vec<TileDraw>,
    // The loaded tilemap's total pixel extent, for `camera_position`'s
    // clamp — `None` before any `load_tilemap` call, or if the `.tmx`
    // couldn't be parsed, in which case a camera-follow centers on its
    // target unclamped rather than pinning to `(0, 0)`.
    level_size_px: Option<(f32, f32)>,
    // `EntityOp::SetCameraFollow` state — `None` is the original fixed
    // `(0, 0)` camera every caller got before this existed.
    camera_follow: Option<CameraFollow>,
}

// Purely `GameCore`'s own internal camera-follow state, never constructed
// or matched outside this file — no independent identity of its own.
struct CameraFollow {
    entity_id: u32,
    viewport_w: f32,
    viewport_h: f32,
    zoom: f32,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            rapier2d: Rapier2dBackend::new(),
            retro: RetroBackend::new(),
            bus: None,
            entities: HashMap::new(),
            entity_ids_by_collider: HashMap::new(),
            debug_hitboxes: false,
            tile_draws: Vec::new(),
            level_size_px: None,
            camera_follow: None,
            paused: false,
        }
    }

    fn backend_mut(&mut self, world: PhysicsWorldKind) -> &mut dyn PhysicsBackend {
        match world {
            PhysicsWorldKind::Rapier2d => &mut self.rapier2d,
            PhysicsWorldKind::Retro => &mut self.retro,
        }
    }

    fn backend(&self, world: PhysicsWorldKind) -> &dyn PhysicsBackend {
        match world {
            PhysicsWorldKind::Rapier2d => &self.rapier2d,
            PhysicsWorldKind::Retro => &self.retro,
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
                shape,
                collider_half_w,
                collider_half_h,
                body_kind,
                worlds,
            } => self.spawn_entity(
                entity_id,
                x,
                y,
                sprite,
                square_color,
                shape,
                collider_half_w,
                collider_half_h,
                body_kind,
                worlds,
            ),
            EntityOp::SetVelocity { entity_id, vx, vy } => self.set_velocity(entity_id, vx, vy),
            EntityOp::Despawn { entity_id } => self.despawn(entity_id),
            EntityOp::SetColor { entity_id, color } => self.set_color(entity_id, color),
            EntityOp::SetDebugHitboxes { enabled } => self.debug_hitboxes = enabled,
            EntityOp::SetPaused { paused } => self.paused = paused,
            EntityOp::SetCameraFollow {
                entity_id,
                viewport_w,
                viewport_h,
                zoom,
            } => {
                self.camera_follow = Some(CameraFollow {
                    entity_id,
                    viewport_w,
                    viewport_h,
                    zoom,
                })
            }
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
        shape: WireShape,
        collider_half_w: f32,
        collider_half_h: f32,
        body_kind: WireBodyKind,
        worlds: PhysicsWorlds,
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
            let physics_shape = match shape {
                WireShape::Rect => Shape::Rect,
                WireShape::Triangle => Shape::Triangle,
            };
            let mut bodies = Vec::new();
            for world in [
                (PhysicsWorldKind::Rapier2d, worlds.rapier2d),
                (PhysicsWorldKind::Retro, worlds.retro),
            ]
            .into_iter()
            .filter_map(|(world, present)| present.then_some(world))
            {
                let (body, collider) = self.backend_mut(world).spawn_shaped_body(
                    Vec2::new(x, y),
                    Vec2::new(collider_half_w, collider_half_h),
                    physics_shape,
                    kind,
                );
                self.entity_ids_by_collider
                    .insert((world, collider), entity_id);
                bodies.push(WorldBody {
                    world,
                    body,
                    collider,
                });
            }
            if !bodies.is_empty() {
                builder.add(Collider {
                    bodies,
                    half_w: collider_half_w,
                    half_h: collider_half_h,
                    shape: physics_shape,
                });
            }
        }

        let entity = self.world.spawn(builder.build());
        self.entities.insert(entity_id, entity);
    }

    /// A no-op if `entity_id` names no entity, or one with no collider —
    /// a purely visual entity has no physics body to set velocity on. Sets
    /// velocity in every world the entity is registered in, not just its
    /// primary one — a lower-priority world's copy still needs to move so
    /// its own same-world collisions stay meaningful, even though its
    /// position is overwritten by the priority snap each tick regardless.
    fn set_velocity(&mut self, entity_id: u32, vx: f32, vy: f32) {
        let Some(&entity) = self.entities.get(&entity_id) else {
            return;
        };
        let Ok(collider) = self.world.get::<&Collider>(entity) else {
            return;
        };
        let bodies = collider.bodies.clone();
        drop(collider);
        for world_body in bodies {
            self.backend_mut(world_body.world)
                .set_velocity(world_body.body, Vec2::new(vx, vy));
        }
    }

    /// A no-op if `entity_id` names no entity — despawning twice, or an id
    /// never spawned, is not an error.
    fn despawn(&mut self, entity_id: u32) {
        let Some(entity) = self.entities.remove(&entity_id) else {
            return;
        };
        let bodies = self
            .world
            .get::<&Collider>(entity)
            .map(|collider| collider.bodies.clone())
            .unwrap_or_default();
        for world_body in bodies {
            self.entity_ids_by_collider
                .remove(&(world_body.world, world_body.collider));
            self.backend_mut(world_body.world)
                .remove_body(world_body.body);
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
    ///
    /// A `"Ground"` tile layer resolves the same way collision does:
    /// parsed once here (`tiles::load_tile_draws`), cached in
    /// `self.tile_draws`, and replayed as `gfx::DrawSprite` every
    /// `publish_gfx` tick rather than re-parsed. Each `TilesetImage` the
    /// caller supplied gets one `gfx::LoadSprite` registered under its own
    /// `sprite_id`; a `.tmx` tileset with no matching image name is parsed
    /// (so its tiles don't break the rest of the layer) but never drawn.
    fn load_tilemap(&mut self, load: LoadTilemap) {
        let Ok(rects) = load_collision_rects(load.tmx_bytes) else {
            return;
        };
        for rect in rects {
            let (body, collider) = self.rapier2d.spawn_body(
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
                    bodies: vec![WorldBody {
                        world: PhysicsWorldKind::Rapier2d,
                        body,
                        collider,
                    }],
                    half_w: rect.half_w,
                    half_h: rect.half_h,
                    shape: Shape::Rect,
                },
            ));
        }

        for image in &load.tileset_images {
            self.publish(LoadSprite {
                id: image.sprite_id,
                png_bytes: image.png_bytes,
            });
        }
        let sprite_id_for_tileset = |name: &str| {
            load.tileset_images
                .iter()
                .find(|image| image.name == name)
                .map(|image| image.sprite_id)
        };
        if let Ok(draws) = load_tile_draws(load.tmx_bytes, sprite_id_for_tileset) {
            self.tile_draws = draws;
        }
        self.level_size_px = map_pixel_size(load.tmx_bytes);
    }

    /// Runs one simulation step, then publishes the resulting frame. While
    /// `paused` (`EntityOp::SetPaused`), skips straight to `publish_gfx`:
    /// neither physics world steps, nothing settles or keeps drifting under
    /// residual velocity, and no `game-core/collision` can fire — every
    /// entity holds exactly its last-unpaused state. `publish_gfx` still
    /// runs regardless, so the frame stays visible (frozen) instead of
    /// going stale or blank.
    fn tick(&mut self, dt: f32) {
        if self.paused {
            self.publish_gfx();
            return;
        }
        self.rapier2d.step(dt);
        self.retro.step(dt);
        self.clamp_velocities_against_contacts();
        self.resolve_multi_world_positions();

        // Animation only advances while actually moving — an entity with
        // no collider (nothing driving it) or one at rest freezes on its
        // current frame instead of animating in place. Reads the primary
        // world's velocity: after `resolve_multi_world_positions`, every
        // world an entity is in was snapped to the same velocity anyway
        // (single-world entities have exactly one to read regardless).
        // Collected into a plain map first (rather than read inline inside
        // `query_mut`) since `self.backend` needs to borrow all of `self`
        // while `query_mut` already holds `self.world` mutably borrowed.
        let moving: HashMap<hecs::Entity, bool> = self
            .world
            .query::<Option<&Collider>>()
            .iter()
            .map(|(entity, collider)| {
                let moving = collider
                    .and_then(|collider| {
                        let primary = collider.primary();
                        self.backend(primary.world).velocity(primary.body)
                    })
                    .is_some_and(|velocity| {
                        velocity.length_squared() > MOVING_SPEED_THRESHOLD_SQUARED
                    });
                (entity, moving)
            })
            .collect();
        for (entity, animation) in self.world.query_mut::<&mut SpriteAnimation>() {
            if moving.get(&entity).copied().unwrap_or(false) {
                animation.advance(dt);
            }
        }

        let translations: HashMap<hecs::Entity, Vec2> = self
            .world
            .query::<&Collider>()
            .iter()
            .filter_map(|(entity, collider)| {
                let primary = collider.primary();
                let translation = self.backend(primary.world).body_translation(primary.body)?;
                Some((entity, translation))
            })
            .collect();
        for (entity, transform) in self.world.query_mut::<&mut Transform>() {
            if let Some(translation) = translations.get(&entity) {
                transform.x = translation.x;
                transform.y = translation.y;
            }
        }

        self.publish_collisions();
        self.publish_gfx();
    }

    /// For every entity registered in more than one physics world
    /// (ADR-021), reads its authoritative position/velocity from the
    /// highest-priority world it's in (`PhysicsWorldKind::PRIORITY`) and
    /// snaps every other world's copy of that same entity to match, so no
    /// world's simulation of a shared entity drifts from what's actually
    /// drawn. A single-world entity is untouched — nothing to resolve.
    fn resolve_multi_world_positions(&mut self) {
        let snaps: Vec<(PhysicsWorldKind, BodyHandle, Vec2, Vec2)> = self
            .world
            .query::<&Collider>()
            .iter()
            .filter(|(_, collider)| collider.bodies.len() > 1)
            .filter_map(|(_, collider)| {
                let primary = collider.primary();
                let position = self.backend(primary.world).body_translation(primary.body)?;
                let velocity = self
                    .backend(primary.world)
                    .velocity(primary.body)
                    .unwrap_or(Vec2::ZERO);
                Some(
                    collider
                        .bodies
                        .iter()
                        .filter(|wb| wb.world != primary.world)
                        .map(move |wb| (wb.world, wb.body, position, velocity))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();
        for (world, body, position, velocity) in snaps {
            self.backend_mut(world)
                .set_body_state(body, position, velocity);
        }
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
        let clamped: Vec<(PhysicsWorldKind, BodyHandle, Vec2)> = self
            .world
            .query::<&Collider>()
            .iter()
            .flat_map(|(_, collider)| collider.bodies.iter())
            .filter_map(|world_body| {
                let backend = self.backend(world_body.world);
                if backend.is_kinematic(world_body.body) {
                    return None;
                }
                let velocity = backend.velocity(world_body.body)?;
                if velocity == Vec2::ZERO {
                    return None;
                }
                let mut clamped_velocity = velocity;
                for normal in backend.contact_normals(world_body.collider) {
                    let inward = clamped_velocity.dot(normal);
                    if inward > 0.0 {
                        clamped_velocity -= normal * inward;
                    }
                }
                (clamped_velocity != velocity)
                    .then_some((world_body.world, world_body.body, clamped_velocity))
            })
            .collect();
        for (world, body, velocity) in clamped {
            self.backend_mut(world).set_velocity(body, velocity);
        }
    }

    /// Translates every contact-start this tick's `step` produced, in
    /// every world, into a `game-core/collision` event, by `ColliderHandle`
    /// — worlds never interact directly (ADR-021: two non-overlapping
    /// worlds), so each is drained and resolved independently. A pair
    /// where either side isn't in `entity_ids_by_collider` (a tilemap
    /// collider, or a collider whose entity was already despawned this
    /// tick) is silently skipped rather than published with a missing id.
    /// `CollisionEvent::Started` alone isn't proof of a real touch —
    /// rapier2d uses speculative contacts, so a pair can appear "Started"
    /// while still a hair's width apart (the mechanism behind real-world
    /// phantom "hit" reports with no visible contact). `has_real_contact`
    /// filters those out by checking the actual contact manifold, not just
    /// the event.
    fn publish_collisions(&mut self) {
        for world in PhysicsWorldKind::PRIORITY {
            let starts = self.backend_mut(world).drain_collision_starts();
            for (collider_a, collider_b) in starts {
                if !self.backend(world).has_real_contact(collider_a, collider_b) {
                    continue;
                }
                let entity_id_a = self
                    .entity_ids_by_collider
                    .get(&(world, collider_a))
                    .copied();
                let entity_id_b = self
                    .entity_ids_by_collider
                    .get(&(world, collider_b))
                    .copied();
                if let (Some(entity_id_a), Some(entity_id_b)) = (entity_id_a, entity_id_b) {
                    self.publish(Collision {
                        entity_id_a,
                        entity_id_b,
                    });
                }
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
        let (camera_x, camera_y, zoom) = self.camera_position();
        self.publish(SetCamera {
            x: camera_x,
            y: camera_y,
            zoom,
        });
        // Ground tiles first, on BACKGROUND_LAYER — resolved once by
        // `load_tilemap`, replayed unchanged every tick (see
        // `tile_draws`'s own field doc comment).
        for draw in &self.tile_draws {
            self.publish(DrawSprite {
                id: draw.sprite_id,
                dst_x: draw.dst_x,
                dst_y: draw.dst_y,
                dst_w: draw.size,
                dst_h: draw.size,
                src_x: draw.src_x,
                src_y: draw.src_y,
                src_w: draw.size,
                src_h: draw.size,
                layer: BACKGROUND_LAYER,
                angle: 0.0,
                flip_h: false,
                flip_v: false,
                tint: (255, 255, 255, 255),
            });
        }
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
            self.publish_shape(transform, collider, true, color.0);
        }
        if self.debug_hitboxes {
            // Every collider-bearing entity, sprite or square alike — a
            // sprite's drawn frame and a square's fill can each differ
            // slightly from what actually collides, which is exactly what
            // this overlay is for checking. Drawn on `ENTITY_LAYER` after
            // every normal draw above, so the outline lands on top.
            for (_, (transform, collider)) in self.world.query::<(&Transform, &Collider)>().iter() {
                self.publish_shape(transform, collider, false, DEBUG_HITBOX_COLOR);
            }
        }
    }

    /// The `gfx::SetCamera` `publish_gfx` publishes every tick: `(0, 0,
    /// 1.0)` (the original fixed unzoomed camera) with no active
    /// `EntityOp::SetCameraFollow`, or if the followed entity doesn't
    /// exist (already despawned, or never spawned) — otherwise centered on
    /// that entity's `Transform` at `follow.zoom`, clamped per axis to
    /// `[0, level_size - viewport_size / zoom]` once `level_size_px` is
    /// known (`load_tilemap` having run), so the camera stops panning once
    /// the level edge would come into view — zooming in shows less world
    /// per pixel, shrinking the effective viewport the clamp uses.
    /// Unclamped (just centered) if no tilemap has been loaded yet, rather
    /// than pinning to `(0, 0)`.
    fn camera_position(&self) -> (f32, f32, f32) {
        let Some(follow) = &self.camera_follow else {
            return (0.0, 0.0, 1.0);
        };
        let Some(&entity) = self.entities.get(&follow.entity_id) else {
            return (0.0, 0.0, 1.0);
        };
        let Ok(transform) = self.world.get::<&Transform>(entity) else {
            return (0.0, 0.0, 1.0);
        };
        let (level_w, level_h) = match self.level_size_px {
            Some((w, h)) => (Some(w), Some(h)),
            None => (None, None),
        };
        let clamp_axis = |center: f32, viewport: f32, level: Option<f32>| -> f32 {
            let raw = center - viewport / 2.0;
            match level {
                Some(level) => raw.clamp(0.0, (level - viewport).max(0.0)),
                None => raw,
            }
        };
        let effective_viewport_w = follow.viewport_w / follow.zoom;
        let effective_viewport_h = follow.viewport_h / follow.zoom;
        (
            clamp_axis(transform.x, effective_viewport_w, level_w),
            clamp_axis(transform.y, effective_viewport_h, level_h),
            follow.zoom,
        )
    }

    /// Publishes one `gfx::DrawRect` or `gfx::DrawTriangle` for `collider`'s
    /// shape at `transform`'s position, `filled` and `color` as given —
    /// shared by `publish_gfx`'s normal per-entity draw and its debug-
    /// hitbox overlay, which differ only in those two parameters.
    fn publish_shape(
        &self,
        transform: &Transform,
        collider: &Collider,
        filled: bool,
        color: (u8, u8, u8, u8),
    ) {
        match collider.shape {
            Shape::Rect => self.publish(DrawRect {
                x: (transform.x - collider.half_w) as i32,
                y: (transform.y - collider.half_h) as i32,
                w: (collider.half_w * 2.0) as u32,
                h: (collider.half_h * 2.0) as u32,
                filled,
                color,
                layer: ENTITY_LAYER,
                screen_space: false,
            }),
            // Matches `Rapier2dBackend::spawn_shaped_body`'s inscribed
            // isoceles triangle: apex centered on top, base along the
            // bottom of the same half-extents box.
            Shape::Triangle => self.publish(DrawTriangle {
                x1: transform.x as i32,
                y1: (transform.y - collider.half_h) as i32,
                x2: (transform.x - collider.half_w) as i32,
                y2: (transform.y + collider.half_h) as i32,
                x3: (transform.x + collider.half_w) as i32,
                y3: (transform.y + collider.half_h) as i32,
                filled,
                color,
                layer: ENTITY_LAYER,
            }),
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
