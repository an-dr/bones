use super::*;

#[test]
fn two_overlapping_dynamic_bodies_separate_after_stepping() {
    let mut physics = Physics::new();

    let body_a = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![0.0, 0.0]));
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_a,
        &mut physics.bodies,
    );

    let body_b = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![0.5, 0.0]));
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_b,
        &mut physics.bodies,
    );

    for _ in 0..60 {
        physics.step(1.0 / 60.0);
    }

    let distance =
        (physics.body_translation(body_b).unwrap() - physics.body_translation(body_a).unwrap()).x;
    assert!(
        distance > 0.5,
        "overlapping bodies should have been pushed apart, got distance {distance}"
    );
}

#[test]
fn a_fixed_body_never_moves() {
    let mut physics = Physics::new();
    let body = physics
        .bodies
        .insert(RigidBodyBuilder::fixed().translation(vector![3.0, 4.0]));
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);

    assert_eq!(physics.body_translation(body), Some(vector![3.0, 4.0]));
}

#[test]
fn an_unknown_handle_returns_none() {
    let mut physics_a = Physics::new();
    let physics_b = Physics::new();
    let stray_handle = physics_a.bodies.insert(RigidBodyBuilder::fixed());

    assert_eq!(physics_b.body_translation(stray_handle), None);
}

#[test]
fn remove_body_takes_the_body_out_of_the_simulation() {
    let mut physics = Physics::new();
    let body = physics.bodies.insert(RigidBodyBuilder::fixed());
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body,
        &mut physics.bodies,
    );

    physics.remove_body(body);

    assert_eq!(physics.body_translation(body), None);
    assert_eq!(
        physics.colliders.len(),
        0,
        "the attached collider should be removed too"
    );
}

#[test]
fn removing_an_already_removed_body_is_a_no_op() {
    let mut physics = Physics::new();
    let body = physics.bodies.insert(RigidBodyBuilder::fixed());
    physics.remove_body(body);

    physics.remove_body(body);
    // Reaching here without panicking is the assertion.
}
