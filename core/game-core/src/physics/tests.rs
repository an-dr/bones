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
fn sustained_driving_velocity_settles_to_shallow_penetration() {
    let mut physics = Physics::new();
    // A fixed obstacle, half-extent 1.0 (a 2x2 square) at the origin.
    let obstacle_body = physics.bodies.insert(RigidBodyBuilder::fixed());
    let obstacle_collider = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        obstacle_body,
        &mut physics.bodies,
    );

    // A dynamic body starting just clear of the obstacle, driven directly
    // into it via a hard-set velocity every tick — the same pattern
    // game-core's own EntityOp::SetVelocity uses for a held-input-driven
    // entity, which is what produced the visibly overlapping squares this
    // increment fixes.
    let pusher_body = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![2.5, 0.0]));
    let pusher_collider = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        pusher_body,
        &mut physics.bodies,
    );

    for _ in 0..120 {
        physics
            .bodies
            .get_mut(pusher_body)
            .unwrap()
            .set_linvel(vector![-5.0, 0.0], true);
        physics.step(1.0 / 60.0);
    }

    let depth = physics.penetration_depth(obstacle_collider, pusher_collider);
    // Half-extents sum to 2.0 (a full overlap would be up to that deep).
    // Measured: rapier2d's untuned defaults settle around 0.0087 here;
    // the tuned stiffness (see Physics::new) settles around 0.0012 — a
    // ~7x reduction. 0.005 draws a clear line between the two without
    // being so tight that ordinary floating-point/solver variance flakes
    // the test.
    assert!(
        depth < 0.005,
        "steady-state penetration under continuous driving force should be shallow, got {depth}"
    );
}

#[test]
fn contact_normals_points_away_from_the_queried_collider() {
    let mut physics = Physics::new();
    // A fixed obstacle to the right of a dynamic pusher — the pusher's
    // contact normal should point further right (away from the obstacle,
    // the direction the pusher must not keep driving into).
    let obstacle_body = physics
        .bodies
        .insert(RigidBodyBuilder::fixed().translation(vector![2.5, 0.0]));
    let obstacle_collider = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        obstacle_body,
        &mut physics.bodies,
    );
    let pusher_body = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![0.6, 0.0]));
    let pusher_collider = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        pusher_body,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);

    let pusher_normals = physics.contact_normals(pusher_collider);
    assert_eq!(pusher_normals.len(), 1, "got {pusher_normals:?}");
    assert!(
        pusher_normals[0].x > 0.9,
        "the pusher's contact normal should point toward the obstacle (away from the pusher), got {:?}",
        pusher_normals[0]
    );

    // The same contact from the obstacle's side should point the other way.
    let obstacle_normals = physics.contact_normals(obstacle_collider);
    assert_eq!(obstacle_normals.len(), 1, "got {obstacle_normals:?}");
    assert!(
        obstacle_normals[0].x < -0.9,
        "the obstacle's contact normal should point away from the pusher, got {:?}",
        obstacle_normals[0]
    );
}

#[test]
fn contact_normals_ignores_speculative_only_contacts() {
    let mut physics = Physics::new();
    let body_a = physics
        .bodies
        .insert(RigidBodyBuilder::fixed().translation(vector![0.0, 0.0]));
    let collider_a = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_a,
        &mut physics.bodies,
    );
    let body_b = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![2.001, 0.0]));
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_b,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);

    assert!(physics.contact_normals(collider_a).is_empty());
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

#[test]
fn drain_collision_starts_reports_a_new_contact_once() {
    let mut physics = Physics::new();
    let body_a = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![0.0, 0.0]));
    let collider_a = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0).active_events(ActiveEvents::COLLISION_EVENTS),
        body_a,
        &mut physics.bodies,
    );
    let body_b = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![0.5, 0.0]));
    let collider_b = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0).active_events(ActiveEvents::COLLISION_EVENTS),
        body_b,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);
    let starts = physics.drain_collision_starts();

    assert_eq!(starts.len(), 1);
    let (a, b) = starts[0];
    assert!(
        (a == collider_a && b == collider_b) || (a == collider_b && b == collider_a),
        "got {starts:?}"
    );

    // Still overlapping, but no *new* start — draining again should be empty.
    physics.step(1.0 / 60.0);
    assert!(physics.drain_collision_starts().is_empty());
}

#[test]
fn has_real_contact_is_true_for_overlapping_colliders() {
    let mut physics = Physics::new();
    // At least one dynamic body: rapier2d only promotes manifold points to
    // active/solver contacts when the pair has a dynamic side — a
    // fixed-fixed pair (neither ever needs force resolution) never gets
    // real contact data populated, regardless of how deeply they overlap.
    let body_a = physics
        .bodies
        .insert(RigidBodyBuilder::fixed().translation(vector![0.0, 0.0]));
    let collider_a = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_a,
        &mut physics.bodies,
    );
    let body_b = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![1.5, 0.0]));
    let collider_b = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_b,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);

    assert!(physics.has_real_contact(collider_a, collider_b));
}

#[test]
fn has_real_contact_is_false_for_colliders_within_the_speculative_margin_only() {
    let mut physics = Physics::new();
    // rapier2d's default prediction_distance is 0.002 units: a gap of
    // 0.001 is inside that margin (rapier2d's narrow phase creates a
    // contact pair for it) but the colliders are not actually touching.
    // At least one dynamic body, same as the "true" case above, so this
    // isolates the margin behavior rather than the fixed-fixed caveat.
    let body_a = physics
        .bodies
        .insert(RigidBodyBuilder::fixed().translation(vector![0.0, 0.0]));
    let collider_a = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_a,
        &mut physics.bodies,
    );
    let body_b = physics
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(vector![2.001, 0.0]));
    let collider_b = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_b,
        &mut physics.bodies,
    );

    physics.step(1.0 / 60.0);

    assert!(!physics.has_real_contact(collider_a, collider_b));
}

#[test]
fn has_real_contact_is_false_for_an_unknown_pair() {
    let mut physics = Physics::new();
    let body_a = physics.bodies.insert(RigidBodyBuilder::fixed());
    let collider_a = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_a,
        &mut physics.bodies,
    );
    let body_b = physics.bodies.insert(RigidBodyBuilder::fixed());
    let collider_b = physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(1.0, 1.0),
        body_b,
        &mut physics.bodies,
    );

    assert!(!physics.has_real_contact(collider_a, collider_b));
}

#[test]
fn colliders_without_active_events_report_no_collision_starts() {
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

    physics.step(1.0 / 60.0);

    assert!(physics.drain_collision_starts().is_empty());
}
