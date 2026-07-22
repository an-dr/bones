use std::sync::{Arc, Mutex};

use super::*;
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, PhysicsWorlds, Shape as WireShape,
    Sprite,
};
use bones_messages::gfx;
use bones_messages::EncodeMessage;
use bus::ServiceRegistry;

/// Records every envelope delivered to it — the minimal way to observe
/// what a module publishes without a real renderer subscribed.
#[derive(Clone, Default)]
struct Spy(Arc<Mutex<Vec<Envelope>>>);

impl Handler for Spy {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().push(envelope.clone());
    }
}

/// Wires `game_core` to a fresh `Bus`, subscribes a `Spy` to `gfx/*` and
/// `game-core/*` (so it also observes published `Collision` events), and
/// returns both plus the bus so a test can call `dispatch()` after
/// `handle`/`tick` to actually deliver what was enqueued (ADR-015:
/// `publish` only enqueues).
fn ready_game_core() -> (GameCore, Bus, Spy) {
    let bus = Bus::new();
    let spy = Spy::default();
    let ep = bus.register("spy", spy.clone());
    ep.subscribe("gfx/*");
    ep.subscribe("game-core/*");

    let mut registry = ServiceRegistry::new();
    registry.provide(bus.clone()).unwrap();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut game_core = GameCore::new();
    game_core.init(&mut ctx).unwrap();

    (game_core, bus, spy)
}

fn envelope(topic: &str, payload: Vec<u8>) -> Envelope {
    Envelope {
        topic: topic.to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload,
    }
}

fn entity_op_envelope(op: EntityOp) -> Envelope {
    envelope(EntityOpMessage::TOPIC, EntityOpMessage(op).encode())
}

fn sprite() -> Sprite {
    Sprite {
        sprite_id: 1,
        frame_w: 16,
        frame_h: 16,
        frame_count: 4,
        frame_duration: 0.1,
    }
}

fn spawn_at(x: f32, y: f32) -> EntityOp {
    spawn_with_id(0, x, y)
}

fn spawn_with_id(entity_id: u32, x: f32, y: f32) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: Some(sprite()),
        square_color: (0, 0, 0, 0),
        shape: WireShape::Rect,
        collider_half_w: 0.0,
        collider_half_h: 0.0,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_with_collider(x: f32, y: f32, half_w: f32, half_h: f32) -> EntityOp {
    spawn_with_collider_and_id(0, x, y, half_w, half_h)
}

fn spawn_with_collider_and_id(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: Some(sprite()),
        square_color: (0, 0, 0, 0),
        shape: WireShape::Rect,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_with_collider_in_worlds(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
    worlds: PhysicsWorlds,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: Some(sprite()),
        square_color: (0, 0, 0, 0),
        shape: WireShape::Rect,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Dynamic,
        worlds,
    }
}

fn spawn_kinematic_with_collider(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: Some(sprite()),
        square_color: (0, 0, 0, 0),
        shape: WireShape::Rect,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Kinematic,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_frictionless_with_collider(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: Some(sprite()),
        square_color: (0, 0, 0, 0),
        shape: WireShape::Rect,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Frictionless,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_square_with_collider(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: (200, 40, 40, 255),
        shape: WireShape::Rect,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_triangle_with_collider(
    entity_id: u32,
    x: f32,
    y: f32,
    half_w: f32,
    half_h: f32,
) -> EntityOp {
    EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: (200, 40, 40, 255),
        shape: WireShape::Triangle,
        collider_half_w: half_w,
        collider_half_h: half_h,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::default(),
    }
}

#[test]
fn a_new_game_core_has_an_empty_world() {
    let game_core = GameCore::new();
    assert_eq!(game_core.world.len(), 0);
}

#[test]
fn name_is_the_bus_endpoint_id() {
    let game_core = GameCore::new();
    assert_eq!(game_core.name(), "game-core");
}

#[test]
fn init_subscribes_game_core_and_tick_topics() {
    let mut registry = ServiceRegistry::new();
    registry.provide(Bus::new()).unwrap();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut game_core = GameCore::new();

    game_core.init(&mut ctx).unwrap();

    assert_eq!(ctx.into_subscriptions(), vec!["game-core/*", "core/tick"]);
}

#[test]
fn init_without_a_bus_service_fails() {
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut game_core = GameCore::new();

    assert!(game_core.init(&mut ctx).is_err());
}

#[test]
fn spawn_with_a_sprite_adds_a_transform_and_animation() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_at(3.0, 4.0)));

    assert_eq!(game_core.world.len(), 1);
    let (_, (transform, animation)) = game_core
        .world
        .query_mut::<(&Transform, &SpriteAnimation)>()
        .into_iter()
        .next()
        .expect("the spawned entity should carry both components");
    assert_eq!(*transform, Transform { x: 3.0, y: 4.0 });
    assert_eq!(animation.sprite_id, 1);
}

#[test]
fn spawn_with_no_sprite_adds_a_square_color_instead() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        1, 0.0, 0.0, 8.0, 8.0,
    )));

    let count = game_core
        .world
        .query_mut::<&SpriteAnimation>()
        .into_iter()
        .count();
    assert_eq!(count, 0, "a square entity should carry no SpriteAnimation");
    let (_, color) = game_core
        .world
        .query_mut::<&SquareColor>()
        .into_iter()
        .next()
        .expect("the spawned entity should carry a SquareColor");
    assert_eq!(color.0, (200, 40, 40, 255));
}

#[test]
fn spawn_entity_with_a_collider_carries_a_collider_component() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider(0.0, 0.0, 1.0, 1.0)));

    let count = game_core.world.query_mut::<&Collider>().into_iter().count();
    assert_eq!(count, 1);
}

#[test]
fn spawn_entity_without_a_collider_carries_none() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_at(0.0, 0.0)));

    let count = game_core.world.query_mut::<&Collider>().into_iter().count();
    assert_eq!(
        count, 0,
        "a zero-size collider request should spawn no physics body"
    );
}

#[test]
fn spawning_with_an_id_already_in_use_replaces_the_entity() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_id(1, 0.0, 0.0)));
    game_core.handle(&entity_op_envelope(spawn_with_id(1, 5.0, 5.0)));

    assert_eq!(
        game_core.world.len(),
        1,
        "the first spawn should have been replaced, not duplicated"
    );
    let (_, transform) = game_core
        .world
        .query_mut::<&Transform>()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(*transform, Transform { x: 5.0, y: 5.0 });
}

#[test]
fn despawn_removes_the_entity_and_its_collider() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));

    game_core.handle(&entity_op_envelope(EntityOp::Despawn { entity_id: 1 }));

    assert_eq!(game_core.world.len(), 0);
    assert_eq!(game_core.rapier2d.body_count(), 0);
}

#[test]
fn despawning_an_unknown_entity_id_is_a_no_op() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(EntityOp::Despawn { entity_id: 99 }));
    // Reaching here without panicking is the assertion.
}

#[test]
fn tick_steps_physics_and_syncs_transforms_for_colliding_entities() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 0.5, 0.0, 1.0, 1.0,
    )));

    for _ in 0..60 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    let mut xs: Vec<f32> = game_core
        .world
        .query_mut::<&Transform>()
        .into_iter()
        .map(|(_, t)| t.x)
        .collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(
        xs[1] - xs[0] > 0.5,
        "overlapping colliders should separate and the sync should reflect it in Transform, got gap {}",
        xs[1] - xs[0]
    );
}

#[test]
fn an_entity_with_no_collider_never_animates() {
    // No collider means nothing to check velocity on, so it never counts
    // as "moving" — a purely visual entity's animation stays frozen.
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_at(0.0, 0.0)));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 0.15 }.encode()));

    let (_, animation) = game_core
        .world
        .query_mut::<&SpriteAnimation>()
        .into_iter()
        .next()
        .expect("the spawned entity should still exist");
    assert_eq!(animation.current_frame(), 0);
}

#[test]
fn a_stationary_collider_bearing_entity_does_not_animate() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 0.15 }.encode()));

    let (_, animation) = game_core
        .world
        .query_mut::<&SpriteAnimation>()
        .into_iter()
        .next()
        .expect("the spawned entity should still exist");
    assert_eq!(animation.current_frame(), 0);
}

#[test]
fn a_moving_collider_bearing_entity_animates() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 10.0,
        vy: 0.0,
    }));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 0.15 }.encode()));

    let (_, animation) = game_core
        .world
        .query_mut::<&SpriteAnimation>()
        .into_iter()
        .next()
        .expect("the spawned entity should still exist");
    assert_eq!(animation.current_frame(), 1);
}

const FIXTURE_TMX: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="4" height="4" tilewidth="16" tileheight="16" infinite="0" nextlayerid="3" nextobjectid="2">
 <layer id="1" name="Ground" width="4" height="4">
  <data encoding="csv">
0,0,0,0,
0,0,0,0,
0,0,0,0,
0,0,0,0
</data>
 </layer>
 <objectgroup id="2" name="Collision">
  <object id="1" x="0" y="0" width="16" height="16"/>
 </objectgroup>
</map>
"#;

#[test]
fn load_tilemap_inserts_a_fixed_collider_per_collision_rect() {
    let mut game_core = GameCore::new();
    let load = LoadTilemap {
        tmx_bytes: FIXTURE_TMX,
    };
    game_core.handle(&envelope(LoadTilemap::TOPIC, load.encode()));

    assert_eq!(game_core.rapier2d.body_count(), 1);
}

#[test]
fn load_tilemap_publishes_a_visible_square_for_its_collider() {
    // Regression test: a tilemap collider used to be a raw rapier2d body
    // with no ECS/gfx presence at all — invisible, so it read as a bug
    // ("invisible wall") rather than an intentional obstacle.
    let (mut game_core, bus, spy) = ready_game_core();
    let load = LoadTilemap {
        tmx_bytes: FIXTURE_TMX,
    };
    game_core.handle(&envelope(LoadTilemap::TOPIC, load.encode()));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let rects: Vec<_> = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .map(|e| gfx::DrawRect::decode(&e.payload).unwrap())
        .collect();

    // The fixture's collision rect is x=0,y=0,w=16,h=16 (top-left), so its
    // center is (8, 8) and its top-left draw position is (0, 0).
    assert_eq!(rects.len(), 1);
    assert_eq!((rects[0].x, rects[0].y), (0, 0));
    assert_eq!((rects[0].w, rects[0].h), (16, 16));
}

#[test]
fn a_tilemap_collider_blocks_an_overlapping_dynamic_entity() {
    let mut game_core = GameCore::new();
    let load = LoadTilemap {
        tmx_bytes: FIXTURE_TMX,
    };
    game_core.handle(&envelope(LoadTilemap::TOPIC, load.encode()));
    // Overlaps the fixture's collider rect centered at (8, 8), half-extent 8.
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 8.0, 8.0, 4.0, 4.0,
    )));

    for _ in 0..60 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    // Looked up by id, not the first `Transform` in the world — the
    // tilemap collider is now its own `Transform`-bearing entity too
    // (rendered visibly, see `load_tilemap`'s doc comment), so iteration
    // order is no longer enough to find the spawned entity.
    let entity = *game_core.entities.get(&1).unwrap();
    let transform = *game_core.world.get::<&Transform>(entity).unwrap();
    assert!(
        transform.x != 8.0 || transform.y != 8.0,
        "the dynamic entity should have been pushed out of the fixed tilemap collider"
    );
}

#[test]
fn tick_publishes_a_clear_a_camera_and_one_draw_sprite_per_sprite_entity() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_at(3.0, 4.0)));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let clears = published
        .iter()
        .filter(|e| e.topic == gfx::Clear::TOPIC)
        .count();
    let cameras = published
        .iter()
        .filter(|e| e.topic == gfx::SetCamera::TOPIC)
        .count();
    let sprites: Vec<_> = published
        .iter()
        .filter(|e| e.topic == gfx::DrawSprite::TOPIC)
        .map(|e| gfx::DrawSprite::decode(&e.payload).unwrap())
        .collect();

    assert_eq!(
        clears, 1,
        "every tick should clear before drawing, or old frames smear"
    );
    assert_eq!(cameras, 1);
    assert_eq!(sprites.len(), 1);
    // Spawned at (3, 4); the sprite fixture is 16x16, so its top-left
    // corner (Transform is the entity's center) is offset by half that.
    assert_eq!((sprites[0].dst_x, sprites[0].dst_y), (-5, -4));
}

#[test]
fn a_sprite_entitys_drawn_position_is_centered_on_its_collider() {
    // Regression test: DrawSprite previously used the collider's center
    // (Transform) directly as its top-left corner, so the visible sprite
    // sat offset from where its collider actually was — contact with
    // other entities looked wrong even though physics was correct.
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 100.0, 100.0, 32.0, 32.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let sprite = published
        .iter()
        .find(|e| e.topic == gfx::DrawSprite::TOPIC)
        .map(|e| gfx::DrawSprite::decode(&e.payload).unwrap())
        .expect("a sprite should have been published");

    // sprite() fixture is 16x16 (half-extent 8), independent of the
    // collider's own (larger) half-extent of 32 — the sprite's own size
    // determines its draw offset, not the collider's.
    assert_eq!((sprite.dst_x, sprite.dst_y), (100 - 8, 100 - 8));
}

#[test]
fn tick_publishes_a_draw_rect_for_a_square_entity() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        1, 10.0, 10.0, 8.0, 8.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let rects: Vec<_> = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .map(|e| gfx::DrawRect::decode(&e.payload).unwrap())
        .collect();

    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0].color, (200, 40, 40, 255));
    assert_eq!((rects[0].w, rects[0].h), (16, 16));
}

#[test]
fn tick_publishes_a_draw_triangle_for_a_triangle_shaped_entity() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_triangle_with_collider(
        1, 10.0, 10.0, 8.0, 8.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let rects = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .count();
    let triangles: Vec<_> = published
        .iter()
        .filter(|e| e.topic == gfx::DrawTriangle::TOPIC)
        .map(|e| gfx::DrawTriangle::decode(&e.payload).unwrap())
        .collect();

    assert_eq!(rects, 0, "a triangle-shaped entity should not draw a rect");
    assert_eq!(triangles.len(), 1);
    assert_eq!(triangles[0].color, (200, 40, 40, 255));
    // Apex centered on top, base along the bottom of the 8.0 half-extents
    // box around (10, 10).
    assert_eq!((triangles[0].x1, triangles[0].y1), (10, 2));
    assert_eq!((triangles[0].x2, triangles[0].y2), (2, 18));
    assert_eq!((triangles[0].x3, triangles[0].y3), (18, 18));
}

#[test]
fn debug_hitboxes_are_off_by_default() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider(
        10.0, 10.0, 8.0, 8.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let yellow_rects = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .map(|e| gfx::DrawRect::decode(&e.payload).unwrap())
        .filter(|rect| rect.color == (255, 255, 0, 255))
        .count();
    assert_eq!(yellow_rects, 0);
}

#[test]
fn enabling_debug_hitboxes_outlines_every_collider_bearing_entity() {
    let (mut game_core, bus, spy) = ready_game_core();
    // A sprite entity and a plain square entity, both with colliders.
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 10.0, 10.0, 8.0, 8.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        2, 20.0, 20.0, 4.0, 4.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetDebugHitboxes {
        enabled: true,
    }));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let yellow_rects: Vec<_> = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .map(|e| gfx::DrawRect::decode(&e.payload).unwrap())
        .filter(|rect| rect.color == (255, 255, 0, 255))
        .collect();

    assert_eq!(yellow_rects.len(), 2);
    assert!(yellow_rects.iter().all(|rect| !rect.filled));
    assert!(yellow_rects.iter().any(|rect| (rect.w, rect.h) == (16, 16)));
    assert!(yellow_rects.iter().any(|rect| (rect.w, rect.h) == (8, 8)));
}

#[test]
fn disabling_debug_hitboxes_stops_the_outline() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider(
        10.0, 10.0, 8.0, 8.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetDebugHitboxes {
        enabled: true,
    }));
    game_core.handle(&entity_op_envelope(EntityOp::SetDebugHitboxes {
        enabled: false,
    }));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let yellow_rects = published
        .iter()
        .filter(|e| e.topic == gfx::DrawRect::TOPIC)
        .map(|e| gfx::DrawRect::decode(&e.payload).unwrap())
        .filter(|rect| rect.color == (255, 255, 0, 255))
        .count();
    assert_eq!(yellow_rects, 0);
}

#[test]
fn a_module_with_no_bus_service_never_panics_on_tick() {
    // `GameCore::new()` directly, bypassing `init` — `bus` stays `None`,
    // exercising the same silent-no-op path a caller that skips `init`
    // (or an `init` that errors before this module is used) would hit.
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_at(0.0, 0.0)));
    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 0.1 }.encode()));
    // Reaching here without panicking is the assertion.
}

#[test]
fn set_velocity_moves_a_collider_bearing_entity_over_time() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        7, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 7,
        vx: 10.0,
        vy: 0.0,
    }));

    for _ in 0..60 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    let (_, transform) = game_core
        .world
        .query_mut::<&Transform>()
        .into_iter()
        .next()
        .expect("the spawned entity should still exist");
    assert!(
        transform.x > 5.0,
        "a 10 units/sec x velocity for 1 second should have moved the entity, got x={}",
        transform.x
    );
}

#[test]
fn continuous_driving_into_an_obstacle_settles_at_shallow_penetration() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        1, 3.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 0.0, 0.0, 1.0, 1.0,
    )));

    // Same pattern game_core_demo actually uses: re-publish SetVelocity
    // every tick from "held input," continuing to command the pusher
    // straight into the obstacle for two full seconds — long enough that
    // the pre-increment-3 bug (velocity re-driven every tick, fighting the
    // solver) would have produced clearly visible overlap.
    for _ in 0..120 {
        game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
            entity_id: 2,
            vx: 5.0,
            vy: 0.0,
        }));
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    let obstacle_entity = *game_core.entities.get(&1).unwrap();
    let obstacle_transform = *game_core.world.get::<&Transform>(obstacle_entity).unwrap();
    let pusher_entity = *game_core.entities.get(&2).unwrap();
    let pusher_transform = *game_core.world.get::<&Transform>(pusher_entity).unwrap();

    // Half-extents sum to 2.0 (centers 2.0 apart is exactly touching, no
    // overlap); the backend's own tuning is verified in isolation by
    // physics::rapier2d_backend's `sustained_driving_velocity_settles_to_
    // shallow_penetration` — this test only checks game-core's
    // contact-clamping logic doesn't reintroduce visible overlap on top
    // of that.
    let separation = obstacle_transform.x - pusher_transform.x;
    assert!(
        separation > 1.99,
        "continuous held-input-style driving into an obstacle should settle at shallow \
         penetration instead of visibly overlapping, got center separation {separation}"
    );
}

#[test]
fn continuous_diagonal_driving_against_a_wall_still_slides_along_it() {
    let mut game_core = GameCore::new();
    // A vertical wall to the right of the pusher's path.
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        1, 3.0, 0.0, 1.0, 10.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 0.0, 0.0, 1.0, 1.0,
    )));

    // Driven diagonally (right and down) into the wall every tick — the
    // rightward component should get blocked once touching, but the
    // downward component (along the wall's free axis) should keep moving
    // the entity, the same "push and slide" a player expects.
    for _ in 0..60 {
        game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
            entity_id: 2,
            vx: 5.0,
            vy: 5.0,
        }));
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    let pusher_entity = *game_core.entities.get(&2).unwrap();
    let transform = *game_core.world.get::<&Transform>(pusher_entity).unwrap();
    assert!(
        transform.y > 2.0,
        "driving diagonally against a wall should still slide along its free axis, got y={}",
        transform.y
    );
}

#[test]
fn set_velocity_for_an_entity_with_no_collider_is_a_no_op() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_id(1, 0.0, 0.0)));

    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 10.0,
        vy: 0.0,
    }));
    // Reaching here without panicking is the assertion.
}

#[test]
fn set_velocity_for_an_unknown_entity_id_is_a_no_op() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 99,
        vx: 10.0,
        vy: 0.0,
    }));
    // Reaching here without panicking is the assertion.
}

#[test]
fn malformed_and_unknown_payloads_are_silently_ignored() {
    let mut game_core = GameCore::new();
    game_core.handle(&envelope(EntityOpMessage::TOPIC, vec![1, 2, 3]));
    game_core.handle(&envelope("game-core/does-not-exist", vec![]));
    // Reaching here without panicking is the assertion.
}

#[test]
fn a_kinematic_body_pushes_a_dynamic_one_without_being_displaced() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_kinematic_with_collider(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 0.5, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 5.0,
        vy: 0.0,
    }));

    for _ in 0..60 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }

    let kinematic_entity = *game_core.entities.get(&1).unwrap();
    let kinematic_transform = *game_core.world.get::<&Transform>(kinematic_entity).unwrap();
    let dynamic_entity = *game_core.entities.get(&2).unwrap();
    let dynamic_transform = *game_core.world.get::<&Transform>(dynamic_entity).unwrap();

    // The kinematic body moved exactly as commanded (5 units/sec for 1
    // second), unaffected by pushing the dynamic body out of its way.
    assert!(
        (kinematic_transform.x - 5.0).abs() < 0.5,
        "the kinematic body should move under its own set velocity, got x={}",
        kinematic_transform.x
    );
    // The dynamic body was pushed further right than the kinematic one
    // advanced to, proving it was displaced rather than merely passed
    // through.
    assert!(
        dynamic_transform.x > kinematic_transform.x,
        "the dynamic body should have been pushed ahead of the kinematic one, got kinematic={} dynamic={}",
        kinematic_transform.x,
        dynamic_transform.x
    );
}

#[test]
fn a_frictionless_body_is_pushed_by_contact_but_stops_almost_immediately() {
    let mut game_core = GameCore::new();
    // A moving dynamic "pusher" approaches a stationary frictionless body
    // from the left.
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_frictionless_with_collider(
        2, 2.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 5.0,
        vy: 0.0,
    }));

    // Drive the pusher into the frictionless body.
    for _ in 0..30 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }
    let frictionless_entity = *game_core.entities.get(&2).unwrap();
    let x_at_contact = game_core
        .world
        .get::<&Transform>(frictionless_entity)
        .unwrap()
        .x;
    assert!(
        x_at_contact > 2.0,
        "the frictionless body should have been pushed off its spawn position, got x={}",
        x_at_contact
    );

    // Stop the pusher and remove it from the frictionless body's path so
    // there's no more contact force, then let a few more ticks pass.
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 0.0,
        vy: 0.0,
    }));
    game_core.handle(&entity_op_envelope(EntityOp::Despawn { entity_id: 1 }));
    for _ in 0..30 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }
    let x_after_settling = game_core
        .world
        .get::<&Transform>(frictionless_entity)
        .unwrap()
        .x;

    // No inertia: once nothing is pushing it, the body should have barely
    // coasted at all rather than drifting on under its last velocity.
    assert!(
        (x_after_settling - x_at_contact).abs() < 0.1,
        "a frictionless body should settle almost immediately once contact ends, got x_at_contact={} x_after_settling={}",
        x_at_contact,
        x_after_settling
    );
}

#[test]
fn two_overlapping_entities_publish_exactly_one_collision_event() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 0.5, 0.0, 1.0, 1.0,
    )));

    // Several ticks: the pair starts in contact and stays in contact while
    // separating — a `Collision` should fire once for the new contact,
    // not once per tick they remain overlapping.
    for _ in 0..10 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let collisions: Vec<_> = published
        .iter()
        .filter(|e| e.topic == Collision::TOPIC)
        .map(|e| Collision::decode(&e.payload).unwrap())
        .collect();

    assert_eq!(collisions.len(), 1, "got {collisions:?}");
    let ids = [collisions[0].entity_id_a, collisions[0].entity_id_b];
    assert!(ids.contains(&1) && ids.contains(&2), "got {ids:?}");
}

#[test]
fn a_non_overlapping_pair_publishes_no_collision_event() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 100.0, 100.0, 1.0, 1.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let collisions = published
        .iter()
        .filter(|e| e.topic == Collision::TOPIC)
        .count();
    assert_eq!(collisions, 0);
}

#[test]
fn colliders_within_the_speculative_margin_but_not_touching_publish_no_collision_event() {
    let (mut game_core, bus, spy) = ready_game_core();
    // rapier2d's default IntegrationParameters::prediction_distance is
    // 0.002 units: two half-extent-1.0 colliders centered 2.0 + 0.001
    // apart have a real gap of 0.001 (inside the speculative margin, so
    // rapier2d's CollisionEvent::Started fires for them) but are not
    // actually touching. Without has_real_contact filtering, this used to
    // publish a phantom Collision.
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 0.0, 0.0, 1.0, 1.0,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        2, 2.001, 0.0, 1.0, 1.0,
    )));

    for _ in 0..5 {
        game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    }
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let collisions = published
        .iter()
        .filter(|e| e.topic == Collision::TOPIC)
        .count();
    assert_eq!(
        collisions, 0,
        "a speculative-only contact (close but not touching) should not publish a Collision"
    );
}

#[test]
fn a_tilemap_collider_never_publishes_a_collision_event() {
    let (mut game_core, bus, spy) = ready_game_core();
    let load = LoadTilemap {
        tmx_bytes: FIXTURE_TMX,
    };
    game_core.handle(&envelope(LoadTilemap::TOPIC, load.encode()));
    // Overlaps the fixture's collider rect centered at (8, 8), half-extent 8.
    game_core.handle(&entity_op_envelope(spawn_with_collider_and_id(
        1, 8.0, 8.0, 4.0, 4.0,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let collisions = published
        .iter()
        .filter(|e| e.topic == Collision::TOPIC)
        .count();
    assert_eq!(
        collisions, 0,
        "a tilemap collider has no entity_id, so a contact with one must never be published"
    );
}

#[test]
fn set_color_overwrites_a_square_entitys_color() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_square_with_collider(
        1, 0.0, 0.0, 8.0, 8.0,
    )));

    game_core.handle(&entity_op_envelope(EntityOp::SetColor {
        entity_id: 1,
        color: (0, 255, 0, 255),
    }));

    let (_, color) = game_core
        .world
        .query_mut::<&SquareColor>()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(color.0, (0, 255, 0, 255));
}

#[test]
fn set_color_for_a_sprite_entity_is_a_no_op() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_id(1, 0.0, 0.0)));

    game_core.handle(&entity_op_envelope(EntityOp::SetColor {
        entity_id: 1,
        color: (0, 255, 0, 255),
    }));
    // Reaching here without panicking is the assertion — a sprite entity
    // has no SquareColor to overwrite.
}

#[test]
fn set_color_for_an_unknown_entity_id_is_a_no_op() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(EntityOp::SetColor {
        entity_id: 99,
        color: (0, 255, 0, 255),
    }));
    // Reaching here without panicking is the assertion.
}

#[test]
fn a_retro_only_entity_registers_no_rapier2d_body() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::RETRO,
    )));

    assert_eq!(game_core.rapier2d.body_count(), 0);
    assert_eq!(game_core.retro.body_count(), 1);
}

#[test]
fn a_retro_only_entity_moves_via_the_retro_backend() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::RETRO,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 10.0,
        vy: 0.0,
    }));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 }.encode()));

    let entity = *game_core.entities.get(&1).unwrap();
    let transform = *game_core.world.get::<&Transform>(entity).unwrap();
    assert_eq!(transform, Transform { x: 10.0, y: 0.0 });
}

#[test]
fn a_dual_world_entitys_transform_is_read_from_the_higher_priority_retro_world() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::BOTH,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 10.0,
        vy: 0.0,
    }));

    // One tick: retro moves exactly `velocity * dt` (10.0 for dt=1.0);
    // rapier2d's dynamic body under the same commanded velocity integrates
    // through its own solver and would not land on exactly the same value
    // — if the drawn transform came from rapier2d instead of retro, this
    // would not match `10.0` exactly.
    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 }.encode()));

    let entity = *game_core.entities.get(&1).unwrap();
    let transform = *game_core.world.get::<&Transform>(entity).unwrap();
    assert_eq!(
        transform,
        Transform { x: 10.0, y: 0.0 },
        "the drawn transform should match retro's exact velocity*dt integration, \
         proving it was read from retro (the higher-priority world), not rapier2d"
    );
}

#[test]
fn a_dual_world_entitys_lower_priority_rapier2d_copy_is_snapped_to_the_retro_position() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::BOTH,
    )));
    game_core.handle(&entity_op_envelope(EntityOp::SetVelocity {
        entity_id: 1,
        vx: 10.0,
        vy: 0.0,
    }));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 }.encode()));

    let entity = *game_core.entities.get(&1).unwrap();
    let collider = game_core.world.get::<&Collider>(entity).unwrap();
    let rapier2d_body = collider.in_world(PhysicsWorldKind::Rapier2d).unwrap().body;
    let retro_body = collider.in_world(PhysicsWorldKind::Retro).unwrap().body;
    drop(collider);

    assert_eq!(
        game_core.rapier2d.body_translation(rapier2d_body),
        game_core.retro.body_translation(retro_body),
        "the lower-priority rapier2d copy should have been snapped to retro's position"
    );
}

#[test]
fn two_overlapping_retro_only_entities_publish_a_collision_event() {
    let (mut game_core, bus, spy) = ready_game_core();
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::RETRO,
    )));
    game_core.handle(&entity_op_envelope(spawn_with_collider_in_worlds(
        2,
        0.5,
        0.0,
        1.0,
        1.0,
        PhysicsWorlds::RETRO,
    )));

    game_core.handle(&envelope(Tick::TOPIC, Tick { dt: 1.0 / 60.0 }.encode()));
    bus.dispatch();

    let published = spy.0.lock().unwrap();
    let collisions: Vec<_> = published
        .iter()
        .filter(|e| e.topic == Collision::TOPIC)
        .map(|e| Collision::decode(&e.payload).unwrap())
        .collect();

    assert_eq!(collisions.len(), 1, "got {collisions:?}");
    let ids = [collisions[0].entity_id_a, collisions[0].entity_id_b];
    assert!(ids.contains(&1) && ids.contains(&2), "got {ids:?}");
}
