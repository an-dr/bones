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
        collider_half_w: 8.0,
        collider_half_h: 8.0,
    }
}

fn spawn_square() -> EntityOp {
    EntityOp::Spawn {
        entity_id: 2,
        x: 0.0,
        y: 0.0,
        sprite: None,
        square_color: (200, 40, 40, 255),
        collider_half_w: 8.0,
        collider_half_h: 8.0,
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
    };
    assert_eq!(LoadTilemap::decode(&load.encode()), Ok(load));
}
