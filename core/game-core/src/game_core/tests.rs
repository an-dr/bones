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
    let spawn = SpawnEntity {
        sprite_id: 1,
        x: 3.0,
        y: 4.0,
        frame_w: 16,
        frame_h: 16,
        frame_count: 4,
        frame_duration: 0.1,
    };
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
fn tick_advances_every_entitys_animation() {
    let mut game_core = GameCore::new();
    let spawn = SpawnEntity {
        sprite_id: 1,
        x: 0.0,
        y: 0.0,
        frame_w: 16,
        frame_h: 16,
        frame_count: 4,
        frame_duration: 0.1,
    };
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
