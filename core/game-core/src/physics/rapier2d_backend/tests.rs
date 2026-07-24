use glam::Vec2;

use super::*;

#[test]
fn two_overlapping_dynamic_bodies_separate_after_stepping() {
    let mut backend = Rapier2dBackend::new();

    let (body_a, _) = backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (body_b, _) = backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    for _ in 0..60 {
        backend.step(1.0 / 60.0);
    }

    let distance =
        (backend.body_translation(body_b).unwrap() - backend.body_translation(body_a).unwrap()).x;
    assert!(
        distance > 0.5,
        "overlapping bodies should have been pushed apart, got distance {distance}"
    );
}

#[test]
fn sustained_driving_velocity_settles_to_shallow_penetration() {
    let mut backend = Rapier2dBackend::new();
    // A fixed obstacle, half-extent 1.0 (a 2x2 square) at the origin.
    let (_, obstacle_collider) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);

    // A dynamic body starting just clear of the obstacle, driven directly
    // into it via a hard-set velocity every tick — the same pattern
    // game-core's own EntityOp::SetVelocity uses for a held-input-driven
    // entity, which is what produced the visibly overlapping squares this
    // tuning fixes.
    let (pusher_body, pusher_collider) =
        backend.spawn_body(Vec2::new(2.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    for _ in 0..120 {
        backend.set_velocity(pusher_body, Vec2::new(-5.0, 0.0));
        backend.step(1.0 / 60.0);
    }

    let depth = backend.penetration_depth(obstacle_collider, pusher_collider);
    // Half-extents sum to 2.0 (a full overlap would be up to that deep).
    // Measured: rapier2d's untuned defaults settle around 0.0087 here;
    // the tuned stiffness (see Rapier2dBackend::new) settles around
    // 0.0012 — a ~7x reduction. 0.005 draws a clear line between the two
    // without being so tight that ordinary floating-point/solver variance
    // flakes the test.
    assert!(
        depth < 0.005,
        "steady-state penetration under continuous driving force should be shallow, got {depth}"
    );
}

#[test]
fn contact_normals_points_away_from_the_queried_collider() {
    let mut backend = Rapier2dBackend::new();
    // A fixed obstacle to the right of a dynamic pusher — the pusher's
    // contact normal should point further right (away from the obstacle,
    // the direction the pusher must not keep driving into).
    let (_, obstacle_collider) =
        backend.spawn_body(Vec2::new(2.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    let (_, pusher_collider) =
        backend.spawn_body(Vec2::new(0.6, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    let pusher_normals = backend.contact_normals(pusher_collider);
    assert_eq!(pusher_normals.len(), 1, "got {pusher_normals:?}");
    assert!(
        pusher_normals[0].x > 0.9,
        "the pusher's contact normal should point toward the obstacle (away from the pusher), got {:?}",
        pusher_normals[0]
    );

    // The same contact from the obstacle's side should point the other way.
    let obstacle_normals = backend.contact_normals(obstacle_collider);
    assert_eq!(obstacle_normals.len(), 1, "got {obstacle_normals:?}");
    assert!(
        obstacle_normals[0].x < -0.9,
        "the obstacle's contact normal should point away from the pusher, got {:?}",
        obstacle_normals[0]
    );
}

#[test]
fn contact_normals_ignores_speculative_only_contacts() {
    let mut backend = Rapier2dBackend::new();
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    backend.spawn_body(Vec2::new(2.001, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(backend.contact_normals(collider_a).is_empty());
}

#[test]
fn a_fixed_body_never_moves() {
    let mut backend = Rapier2dBackend::new();
    let (body, _) = backend.spawn_body(Vec2::new(3.0, 4.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);

    backend.step(1.0 / 60.0);

    assert_eq!(backend.body_translation(body), Some(Vec2::new(3.0, 4.0)));
}

#[test]
fn an_unknown_handle_returns_none() {
    let mut backend_a = Rapier2dBackend::new();
    let backend_b = Rapier2dBackend::new();
    let (stray_handle, _) =
        backend_a.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);

    assert_eq!(backend_b.body_translation(stray_handle), None);
}

#[test]
fn remove_body_takes_the_body_out_of_the_simulation() {
    let mut backend = Rapier2dBackend::new();
    let (body, collider) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);

    backend.remove_body(body);

    assert_eq!(backend.body_translation(body), None);
    assert!(!backend.has_real_contact(collider, collider));
}

#[test]
fn removing_an_already_removed_body_is_a_no_op() {
    let mut backend = Rapier2dBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);
    backend.remove_body(body);

    backend.remove_body(body);
    // Reaching here without panicking is the assertion.
}

#[test]
fn drain_collision_starts_reports_a_new_contact_once() {
    let mut backend = Rapier2dBackend::new();
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (_, collider_b) =
        backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);
    let starts = backend.drain_collision_starts();

    assert_eq!(starts.len(), 1);
    let (a, b) = starts[0];
    assert!(
        (a == collider_a && b == collider_b) || (a == collider_b && b == collider_a),
        "got {starts:?}"
    );

    // Still overlapping, but no *new* start — draining again should be empty.
    backend.step(1.0 / 60.0);
    assert!(backend.drain_collision_starts().is_empty());
}

#[test]
fn has_real_contact_is_true_for_overlapping_colliders() {
    let mut backend = Rapier2dBackend::new();
    // At least one dynamic body: rapier2d only promotes manifold points to
    // active/solver contacts when the pair has a dynamic side — a
    // fixed-fixed pair (neither ever needs force resolution) never gets
    // real contact data populated, regardless of how deeply they overlap.
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    let (_, collider_b) =
        backend.spawn_body(Vec2::new(1.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(backend.has_real_contact(collider_a, collider_b));
}

#[test]
fn has_real_contact_is_false_for_colliders_within_the_speculative_margin_only() {
    let mut backend = Rapier2dBackend::new();
    // rapier2d's default prediction_distance is 0.002 units: a gap of
    // 0.001 is inside that margin (rapier2d's narrow phase creates a
    // contact pair for it) but the colliders are not actually touching.
    // At least one dynamic body, same as the "true" case above, so this
    // isolates the margin behavior rather than the fixed-fixed caveat.
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    let (_, collider_b) =
        backend.spawn_body(Vec2::new(2.001, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(!backend.has_real_contact(collider_a, collider_b));
}

#[test]
fn has_real_contact_is_false_for_an_unknown_pair() {
    let mut backend = Rapier2dBackend::new();
    let (_, collider_a) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);
    let (_, collider_b) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);

    assert!(!backend.has_real_contact(collider_a, collider_b));
}

#[test]
fn colliders_without_active_events_report_no_collision_starts() {
    // Every `spawn_body`-created collider requests
    // `ActiveEvents::COLLISION_EVENTS` unconditionally (unlike the old
    // direct-rapier2d tilemap path, which didn't) — this backend has no
    // way to spawn a collider that opts out. Two non-overlapping bodies
    // are used instead to exercise the same "no start reported" outcome.
    let mut backend = Rapier2dBackend::new();
    backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.spawn_body(Vec2::new(10.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(backend.drain_collision_starts().is_empty());
}

#[test]
fn a_kinematic_body_reports_as_kinematic() {
    let mut backend = Rapier2dBackend::new();
    let (kinematic_body, _) =
        backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Kinematic);
    let (dynamic_body, _) =
        backend.spawn_body(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    assert!(backend.is_kinematic(kinematic_body));
    assert!(!backend.is_kinematic(dynamic_body));
}

#[test]
fn set_body_state_overwrites_position_and_velocity() {
    let mut backend = Rapier2dBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.set_velocity(body, Vec2::new(9.0, 9.0));

    backend.set_body_state(body, Vec2::new(7.0, 8.0), Vec2::new(1.0, -1.0));

    assert_eq!(backend.body_translation(body), Some(Vec2::new(7.0, 8.0)));
    assert_eq!(backend.velocity(body), Some(Vec2::new(1.0, -1.0)));
}
