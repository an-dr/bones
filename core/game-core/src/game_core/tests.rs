use super::*;

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
