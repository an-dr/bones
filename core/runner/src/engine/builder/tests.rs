use super::*;

#[test]
fn tick_hz_defaults_to_60_and_is_overridable() {
    assert_eq!(Engine::new().tick_hz, DEFAULT_TICK_HZ);
    assert_eq!(Engine::new().tick_hz(30.0).tick_hz, 30.0);
}
