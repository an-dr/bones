use super::*;
use rapier2d::prelude::{ColliderSet, RigidBodySet};

#[test]
fn equal_handles_are_equal() {
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let body = bodies.insert(rapier2d::prelude::RigidBodyBuilder::fixed());
    let collider = colliders.insert(rapier2d::prelude::ColliderBuilder::cuboid(1.0, 1.0));

    let a = Collider {
        body,
        collider,
        half_w: 1.0,
        half_h: 1.0,
    };
    let b = Collider {
        body,
        collider,
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(a, b);
}
