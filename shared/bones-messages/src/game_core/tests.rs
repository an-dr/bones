use super::*;
use crate::{DecodeError, EncodeMessage, Message};

#[test]
fn non_game_core_topics_are_ignored() {
    assert_eq!(Command::decode("input/key-down", b"whatever"), Ok(None));
}

#[test]
fn spawn_entity_round_trips() {
    let spawn = SpawnEntity {
        sprite_id: 3,
        x: 10.5,
        y: -4.0,
        frame_w: 16,
        frame_h: 16,
        frame_count: 4,
        frame_duration: 0.1,
        collider_half_w: 8.0,
        collider_half_h: 8.0,
    };
    assert_eq!(
        Command::decode(SpawnEntity::TOPIC, &spawn.encode()),
        Ok(Some(Command::SpawnEntity(spawn)))
    );
}

#[test]
fn truncated_payload_is_rejected() {
    assert_eq!(
        Command::decode(SpawnEntity::TOPIC, &[0; 3]),
        Err(DecodeError::Truncated)
    );
}
