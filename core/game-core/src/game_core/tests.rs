use super::*;
use bones_messages::EncodeMessage;
use bus::ServiceRegistry;

fn envelope(topic: &str, payload: Vec<u8>) -> Envelope {
    Envelope {
        topic: topic.to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload,
    }
}

fn spawn_at(x: f32, y: f32) -> SpawnEntity {
    SpawnEntity {
        sprite_id: 1,
        x,
        y,
        frame_w: 16,
        frame_h: 16,
        frame_count: 4,
        frame_duration: 0.1,
        collider_half_w: 0.0,
        collider_half_h: 0.0,
    }
}

fn spawn_with_collider(x: f32, y: f32, half_w: f32, half_h: f32) -> SpawnEntity {
    SpawnEntity {
        collider_half_w: half_w,
        collider_half_h: half_h,
        ..spawn_at(x, y)
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
    let mut ctx = ModuleContext::new(&mut registry);
    let mut game_core = GameCore::new();

    game_core.init(&mut ctx).unwrap();

    assert_eq!(ctx.into_subscriptions(), vec!["game-core/*", "core/tick"]);
}

#[test]
fn spawn_entity_adds_a_transform_and_animation() {
    let mut game_core = GameCore::new();
    let spawn = spawn_at(3.0, 4.0);
    game_core.handle(&envelope(SpawnEntity::TOPIC, spawn.encode()));

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
fn spawn_entity_with_a_collider_carries_a_collider_component() {
    let mut game_core = GameCore::new();
    let spawn = spawn_with_collider(0.0, 0.0, 1.0, 1.0);
    game_core.handle(&envelope(SpawnEntity::TOPIC, spawn.encode()));

    let count = game_core.world.query_mut::<&Collider>().into_iter().count();
    assert_eq!(count, 1);
}

#[test]
fn spawn_entity_without_a_collider_carries_none() {
    let mut game_core = GameCore::new();
    let spawn = spawn_at(0.0, 0.0);
    game_core.handle(&envelope(SpawnEntity::TOPIC, spawn.encode()));

    let count = game_core.world.query_mut::<&Collider>().into_iter().count();
    assert_eq!(
        count, 0,
        "a zero-size collider request should spawn no physics body"
    );
}

#[test]
fn tick_steps_physics_and_syncs_transforms_for_colliding_entities() {
    let mut game_core = GameCore::new();
    game_core.handle(&envelope(
        SpawnEntity::TOPIC,
        spawn_with_collider(0.0, 0.0, 1.0, 1.0).encode(),
    ));
    game_core.handle(&envelope(
        SpawnEntity::TOPIC,
        spawn_with_collider(0.5, 0.0, 1.0, 1.0).encode(),
    ));

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
    let spawn = spawn_at(0.0, 0.0);
    game_core.handle(&envelope(SpawnEntity::TOPIC, spawn.encode()));

    let tick = Tick { dt: 0.15 };
    game_core.handle(&envelope(Tick::TOPIC, tick.encode()));

    let (_, animation) = game_core
        .world
        .query_mut::<&SpriteAnimation>()
        .into_iter()
        .next()
        .expect("the spawned entity should still exist");
    assert_eq!(animation.current_frame(), 1);
}

#[test]
fn malformed_and_unknown_payloads_are_silently_ignored() {
    let mut game_core = GameCore::new();
    game_core.handle(&envelope("game-core/spawn-entity", vec![1, 2, 3]));
    game_core.handle(&envelope("game-core/does-not-exist", vec![]));
    // Reaching here without panicking is the assertion.
}
