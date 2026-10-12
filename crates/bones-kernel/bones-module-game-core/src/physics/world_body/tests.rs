use super::*;

#[test]
fn equal_fields_are_equal() {
    let a = WorldBody {
        world: PhysicsWorldKind::Retro,
        body: BodyHandle(1),
        collider: ColliderHandle(1),
    };
    let b = WorldBody {
        world: PhysicsWorldKind::Retro,
        body: BodyHandle(1),
        collider: ColliderHandle(1),
    };
    assert_eq!(a, b);
}
