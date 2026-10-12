use super::*;

#[test]
fn a_body_moves_by_velocity_times_dt() {
    let mut backend = RetroBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.set_velocity(body, Vec2::new(10.0, 0.0));

    backend.step(1.0);

    assert_eq!(backend.body_translation(body), Some(Vec2::new(10.0, 0.0)));
}

#[test]
fn a_fixed_body_never_moves_even_with_a_set_velocity() {
    let mut backend = RetroBackend::new();
    let (body, _) = backend.spawn_body(Vec2::new(3.0, 4.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    backend.set_velocity(body, Vec2::new(10.0, 0.0));

    backend.step(1.0);

    assert_eq!(backend.body_translation(body), Some(Vec2::new(3.0, 4.0)));
}

#[test]
fn two_overlapping_dynamic_bodies_are_pushed_apart_along_the_shallow_axis() {
    let mut backend = RetroBackend::new();
    // Overlap is 1.5 units on x (half-extents sum 2.0 - delta 0.5), 2.0 on
    // y (delta 0.0) — x is the shallower axis, so separation should be
    // purely horizontal.
    let (body_a, _) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (body_b, _) =
        backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    let pos_a = backend.body_translation(body_a).unwrap();
    let pos_b = backend.body_translation(body_b).unwrap();
    assert_eq!(pos_a.y, 0.0, "no vertical separation expected");
    assert_eq!(pos_b.y, 0.0, "no vertical separation expected");
    assert!(
        pos_b.x - pos_a.x > 0.5,
        "overlapping bodies should have been pushed apart on x, got {} vs {}",
        pos_a.x,
        pos_b.x
    );
}

#[test]
fn a_dynamic_body_pushed_against_a_fixed_one_moves_alone() {
    let mut backend = RetroBackend::new();
    let (fixed_body, _) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Fixed);
    let (dynamic_body, _) =
        backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert_eq!(
        backend.body_translation(fixed_body),
        Some(Vec2::new(0.0, 0.0)),
        "a fixed body must never be displaced by separation"
    );
    assert!(
        backend.body_translation(dynamic_body).unwrap().x > 0.5,
        "the dynamic body should carry the entire separation"
    );
}

#[test]
fn a_dynamic_body_pushed_against_a_kinematic_one_moves_alone() {
    let mut backend = RetroBackend::new();
    let (kinematic_body, _) = backend.spawn_body(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        BodyKind::Kinematic,
    );
    let (dynamic_body, _) =
        backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert_eq!(
        backend.body_translation(kinematic_body),
        Some(Vec2::new(0.0, 0.0)),
        "a kinematic body must never be displaced by separation"
    );
    assert!(backend.body_translation(dynamic_body).unwrap().x > 0.5);
}

#[test]
fn a_moving_kinematic_body_pushes_a_dynamic_one_out_of_its_way() {
    let mut backend = RetroBackend::new();
    let (kinematic_body, _) = backend.spawn_body(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        BodyKind::Kinematic,
    );
    let (dynamic_body, _) =
        backend.spawn_body(Vec2::new(1.9, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.set_velocity(kinematic_body, Vec2::new(5.0, 0.0));

    for _ in 0..30 {
        backend.step(1.0 / 60.0);
    }

    let kinematic_pos = backend.body_translation(kinematic_body).unwrap();
    let dynamic_pos = backend.body_translation(dynamic_body).unwrap();
    assert!(
        dynamic_pos.x > 1.9,
        "the dynamic body should have been shoved forward, got {}",
        dynamic_pos.x
    );
    assert!(
        dynamic_pos.x - kinematic_pos.x >= 1.999,
        "the kinematic body should never end up overlapping the one it pushed, got centers {} vs {}",
        kinematic_pos.x,
        dynamic_pos.x
    );
}

#[test]
fn non_overlapping_bodies_are_not_moved_by_step() {
    let mut backend = RetroBackend::new();
    let (body_a, _) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (body_b, _) =
        backend.spawn_body(Vec2::new(10.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert_eq!(backend.body_translation(body_a), Some(Vec2::new(0.0, 0.0)));
    assert_eq!(backend.body_translation(body_b), Some(Vec2::new(10.0, 0.0)));
}

#[test]
fn drain_collision_starts_reports_a_new_contact_once() {
    let mut backend = RetroBackend::new();
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

    // Separation from the first step should already have resolved the
    // overlap (or at least not be a *new* start) — draining again after a
    // second step should report no further start for the same pair.
    backend.step(1.0 / 60.0);
    assert!(backend.drain_collision_starts().is_empty());
}

#[test]
fn has_real_contact_is_false_for_a_pair_that_never_touched() {
    let mut backend = RetroBackend::new();
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (_, collider_b) =
        backend.spawn_body(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    assert!(!backend.has_real_contact(collider_a, collider_b));
}

#[test]
fn has_real_contact_is_true_right_after_a_step_that_found_real_overlap() {
    // Regression: `step` immediately separates overlapping pushable bodies
    // in the same call, so by the time a caller can ask, a fresh geometric
    // overlap check against the *current* (already-separated) positions
    // would wrongly read false for a contact that was completely real —
    // `has_real_contact` must reflect what `step` actually found, not the
    // post-separation position.
    let mut backend = RetroBackend::new();
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    let (_, collider_b) =
        backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(backend.has_real_contact(collider_a, collider_b));
}

#[test]
fn contact_normals_is_always_empty() {
    let mut backend = RetroBackend::new();
    let (_, collider_a) =
        backend.spawn_body(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.spawn_body(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.step(1.0 / 60.0);

    assert!(backend.contact_normals(collider_a).is_empty());
}

#[test]
fn remove_body_takes_the_body_out_of_the_simulation() {
    let mut backend = RetroBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    backend.remove_body(body);

    assert_eq!(backend.body_translation(body), None);
    assert_eq!(backend.body_count(), 0);
}

#[test]
fn removing_an_already_removed_body_is_a_no_op() {
    let mut backend = RetroBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.remove_body(body);

    backend.remove_body(body);
    // Reaching here without panicking is the assertion.
}

#[test]
fn a_kinematic_body_reports_as_kinematic() {
    let mut backend = RetroBackend::new();
    let (kinematic_body, _) =
        backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Kinematic);
    let (dynamic_body, _) =
        backend.spawn_body(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0), BodyKind::Dynamic);

    assert!(backend.is_kinematic(kinematic_body));
    assert!(!backend.is_kinematic(dynamic_body));
}

#[test]
fn set_body_state_overwrites_position_and_velocity() {
    let mut backend = RetroBackend::new();
    let (body, _) = backend.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Dynamic);
    backend.set_velocity(body, Vec2::new(9.0, 9.0));

    backend.set_body_state(body, Vec2::new(7.0, 8.0), Vec2::new(1.0, -1.0));

    assert_eq!(backend.body_translation(body), Some(Vec2::new(7.0, 8.0)));
    assert_eq!(backend.velocity(body), Some(Vec2::new(1.0, -1.0)));
}

#[test]
fn an_unknown_handle_returns_none() {
    let mut backend_a = RetroBackend::new();
    let backend_b = RetroBackend::new();
    let (stray_handle, _) = backend_a.spawn_body(Vec2::ZERO, Vec2::new(1.0, 1.0), BodyKind::Fixed);

    assert_eq!(backend_b.body_translation(stray_handle), None);
}
