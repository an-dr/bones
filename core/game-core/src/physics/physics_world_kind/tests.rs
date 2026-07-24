use super::*;

#[test]
fn retro_outranks_rapier2d() {
    assert_eq!(
        PhysicsWorldKind::PRIORITY,
        [PhysicsWorldKind::Retro, PhysicsWorldKind::Rapier2d]
    );
}
