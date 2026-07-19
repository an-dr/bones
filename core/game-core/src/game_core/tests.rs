use std::sync::{Arc, Mutex};

use super::*;
use bones_messages::game_core::{EntityOp, EntityOpMessage, LoadTilemap, Sprite};
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

/// Wires `game_core` to a fresh `Bus`, subscribes a `Spy` to `gfx/*`, and
/// returns both plus the bus so a test can call `dispatch()` after
/// `handle`/`tick` to actually deliver what was enqueued (ADR-015:
/// `publish` only enqueues).
fn ready_game_core() -> (GameCore, Bus, Spy) {
    let bus = Bus::new();
    let spy = Spy::default();
    let ep = bus.register("spy", spy.clone());
    ep.subscribe("gfx/*");

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
        collider_half_w: 0.0,
        collider_half_h: 0.0,
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
        collider_half_w: half_w,
        collider_half_h: half_h,
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
        collider_half_w: half_w,
        collider_half_h: half_h,
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
    assert_eq!(game_core.physics.bodies.len(), 0);
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
fn tick_advances_every_entitys_animation() {
    let mut game_core = GameCore::new();
    game_core.handle(&entity_op_envelope(spawn_at(0.0, 0.0)));

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

    assert_eq!(game_core.physics.bodies.len(), 1);
}

#[test]
fn a_tilemap_collider_blocks_an_overlapping_dynamic_entity() {
    let mut game_core = GameCore::new();
    let load = LoadTilemap {
        tmx_bytes: FIXTURE_TMX,
    };
    game_core.handle(&envelope(LoadTilemap::TOPIC, load.encode()));
    // Overlaps the fixture's collider rect centered at (8, 8), half-extent 8.
    game_core.handle(&entity_op_envelope(spawn_with_collider(8.0, 8.0, 4.0, 4.0)));

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
    assert_eq!((sprites[0].dst_x, sprites[0].dst_y), (3, 4));
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
