use super::*;
use rapier2d::prelude::{ColliderSet, RigidBodySet};

#[test]
fn equal_handles_are_equal() {
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let body = bodies.insert(rapier2d::prelude::RigidBodyBuilder::fixed());
    let collider = colliders.insert(rapier2d::prelude::ColliderBuilder::cuboid(1.0, 1.0));

    assert_eq!(Collider { body, collider }, Collider { body, collider });
}
