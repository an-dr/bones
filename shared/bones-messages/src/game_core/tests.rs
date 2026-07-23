use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage};

fn spawn_with_sprite() -> EntityOp {
    EntityOp::Spawn {
        entity_id: 1,
        x: 10.5,
        y: -4.0,
        sprite: Some(Sprite {
            sprite_id: 3,
            frame_w: 16,
            frame_h: 16,
            frame_count: 4,
            frame_duration: 0.1,
        }),
        square_color: (0, 0, 0, 0),
        shape: Shape::Rect,
        collider_half_w: 8.0,
        collider_half_h: 8.0,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::default(),
    }
}

fn spawn_square() -> EntityOp {
    EntityOp::Spawn {
        entity_id: 2,
        x: 0.0,
        y: 0.0,
        sprite: None,
        square_color: (200, 40, 40, 255),
        shape: Shape::Rect,
        collider_half_w: 8.0,
        collider_half_h: 8.0,
        body_kind: BodyKind::Kinematic,
        worlds: PhysicsWorlds::default(),
    }
}

#[test]
fn spawn_with_sprite_round_trips() {
    let op = spawn_with_sprite();
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn spawn_square_with_no_sprite_round_trips() {
    let op = spawn_square();
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn spawn_with_frictionless_body_kind_round_trips() {
    let op = EntityOp::Spawn {
        entity_id: 2,
        x: 0.0,
        y: 0.0,
        sprite: None,
        square_color: (60, 120, 220, 255),
        shape: Shape::Rect,
        collider_half_w: 8.0,
        collider_half_h: 8.0,
        body_kind: BodyKind::Frictionless,
        worlds: PhysicsWorlds::default(),
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn spawn_registered_in_both_physics_worlds_round_trips() {
    let op = EntityOp::Spawn {
        entity_id: 5,
        x: 1.0,
        y: 2.0,
        sprite: None,
        square_color: (10, 20, 30, 255),
        shape: Shape::Rect,
        collider_half_w: 4.0,
        collider_half_h: 4.0,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::BOTH,
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn spawn_registered_in_retro_only_round_trips() {
    let op = EntityOp::Spawn {
        entity_id: 6,
        x: 0.0,
        y: 0.0,
        sprite: None,
        square_color: (0, 0, 0, 255),
        shape: Shape::Rect,
        collider_half_w: 2.0,
        collider_half_h: 2.0,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::RETRO,
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn spawn_with_triangle_shape_round_trips() {
    let op = EntityOp::Spawn {
        entity_id: 8,
        x: 3.0,
        y: 4.0,
        sprite: None,
        square_color: (200, 60, 60, 255),
        shape: Shape::Triangle,
        collider_half_w: 20.0,
        collider_half_h: 20.0,
        body_kind: BodyKind::Kinematic,
        worlds: PhysicsWorlds::default(),
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_velocity_round_trips() {
    let op = EntityOp::SetVelocity {
        entity_id: 1,
        vx: -3.5,
        vy: 2.0,
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn despawn_round_trips() {
    let op = EntityOp::Despawn { entity_id: 7 };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_color_round_trips() {
    let op = EntityOp::SetColor {
        entity_id: 4,
        color: (255, 0, 128, 255),
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_debug_hitboxes_enabled_round_trips() {
    let op = EntityOp::SetDebugHitboxes { enabled: true };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_debug_hitboxes_disabled_round_trips() {
    let op = EntityOp::SetDebugHitboxes { enabled: false };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_paused_enabled_round_trips() {
    let op = EntityOp::SetPaused { paused: true };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_paused_disabled_round_trips() {
    let op = EntityOp::SetPaused { paused: false };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn set_camera_follow_round_trips() {
    let op = EntityOp::SetCameraFollow {
        entity_id: 1,
        viewport_w: 800.0,
        viewport_h: 600.0,
        zoom: 1.5,
    };
    let message = EntityOpMessage(op);
    assert_eq!(
        EntityOpMessage::decode(&message.encode()),
        Ok(EntityOpMessage(op))
    );
}

#[test]
fn an_invalid_tag_is_rejected() {
    assert_eq!(
        EntityOpMessage::decode(&[255]),
        Err(DecodeError::InvalidTag {
            message: "game-core entity op",
            tag: 255
        })
    );
}

#[test]
fn truncated_payload_is_rejected() {
    assert_eq!(
        EntityOpMessage::decode(&[0, 0, 0]),
        Err(DecodeError::Truncated)
    );
}

#[test]
fn load_tilemap_round_trips() {
    let load = LoadTilemap {
        tmx_bytes: b"<map></map>",
        tileset_images: Vec::new(),
    };
    assert_eq!(LoadTilemap::decode(&load.encode()), Ok(load));
}

#[test]
fn load_tilemap_with_tileset_images_round_trips() {
    let load = LoadTilemap {
        tmx_bytes: b"<map></map>",
        tileset_images: vec![
            TilesetImage {
                name: "grass",
                sprite_id: 2,
                png_bytes: b"not-really-a-png",
            },
            TilesetImage {
                name: "bricks",
                sprite_id: 3,
                png_bytes: b"also-not-really-a-png",
            },
        ],
    };
    assert_eq!(LoadTilemap::decode(&load.encode()), Ok(load));
}

#[test]
fn collision_round_trips() {
    let collision = Collision {
        entity_id_a: 3,
        entity_id_b: 7,
    };
    assert_eq!(Collision::decode(&collision.encode()), Ok(collision));
}
