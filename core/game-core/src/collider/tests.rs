use physics::{BodyHandle, ColliderHandle};

use super::*;

fn world_body(world: PhysicsWorldKind, id: u64) -> WorldBody {
    WorldBody {
        world,
        body: BodyHandle(id),
        collider: ColliderHandle(id),
    }
}

#[test]
fn equal_handles_are_equal() {
    let a = Collider {
        bodies: vec![world_body(PhysicsWorldKind::Rapier2d, 1)],
        half_w: 1.0,
        half_h: 1.0,
    };
    let b = Collider {
        bodies: vec![world_body(PhysicsWorldKind::Rapier2d, 1)],
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(a, b);
}

#[test]
fn in_world_finds_a_registered_world() {
    let collider = Collider {
        bodies: vec![world_body(PhysicsWorldKind::Retro, 5)],
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(
        collider.in_world(PhysicsWorldKind::Retro),
        Some(&world_body(PhysicsWorldKind::Retro, 5))
    );
}

#[test]
fn in_world_returns_none_for_an_unregistered_world() {
    let collider = Collider {
        bodies: vec![world_body(PhysicsWorldKind::Retro, 5)],
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(collider.in_world(PhysicsWorldKind::Rapier2d), None);
}

#[test]
fn primary_prefers_retro_over_rapier2d() {
    let collider = Collider {
        bodies: vec![
            world_body(PhysicsWorldKind::Rapier2d, 1),
            world_body(PhysicsWorldKind::Retro, 2),
        ],
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(collider.primary().world, PhysicsWorldKind::Retro);
}

#[test]
fn primary_falls_back_to_the_only_world_present() {
    let collider = Collider {
        bodies: vec![world_body(PhysicsWorldKind::Rapier2d, 1)],
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(collider.primary().world, PhysicsWorldKind::Rapier2d);
}
