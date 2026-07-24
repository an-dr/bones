use super::*;

#[test]
fn default_is_rapier2d_only() {
    assert_eq!(PhysicsWorlds::default(), PhysicsWorlds::RAPIER2D);
}

#[test]
fn bits_round_trip_every_combination() {
    for worlds in [
        PhysicsWorlds {
            rapier2d: false,
            retro: false,
        },
        PhysicsWorlds::RAPIER2D,
        PhysicsWorlds::RETRO,
        PhysicsWorlds::BOTH,
    ] {
        assert_eq!(PhysicsWorlds::from_bits(worlds.to_bits()), worlds);
    }
}
