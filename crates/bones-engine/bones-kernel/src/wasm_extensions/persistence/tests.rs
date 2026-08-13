use super::*;
use crate::bus::ServiceRegistry;

#[test]
fn init_creates_the_save_directory_and_subscribes_persistence_topics() {
    let dir = std::env::temp_dir().join("bones-persistence-test-init");
    std::fs::remove_dir_all(&dir).ok();
    assert!(!dir.exists(), "test setup: directory must not pre-exist");

    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut persistence = Persistence::new(&dir, false);

    persistence
        .init(&mut ctx)
        .expect("creating a scratch directory should succeed");

    assert!(dir.is_dir());
    assert_eq!(ctx.into_subscriptions(), vec!["persistence/*"]);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_is_fine_with_an_already_existing_directory() {
    let dir = std::env::temp_dir().join("bones-persistence-test-existing");
    std::fs::create_dir_all(&dir).unwrap();

    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut persistence = Persistence::new(&dir, false);

    persistence
        .init(&mut ctx)
        .expect("an already-existing directory is not an error");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn name_is_persistence() {
    assert_eq!(Persistence::new("saves", false).name(), "persistence");
}

fn ready(dir: &std::path::Path, read_only: bool) -> Persistence {
    std::fs::remove_dir_all(dir).ok();
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut persistence = Persistence::new(dir, read_only);
    persistence.init(&mut ctx).unwrap();
    persistence
}

fn envelope(topic: &str, sender: &str, payload: Vec<u8>) -> Envelope {
    Envelope {
        topic: topic.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload,
    }
}

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let dir = std::env::temp_dir().join("bones-persistence-test-round-trip");
    let mut persistence = ready(&dir, false);

    persistence.handle(&envelope(
        Save::TOPIC,
        "sprite_demo",
        bones_messages::EncodeMessage::encode(&Save {
            bytes: b"level=3;hp=42",
        }),
    ));

    let loaded = persistence.respond("sprite_demo", &[]);

    assert_eq!(loaded, Some(b"level=3;hp=42".to_vec()));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loading_before_ever_saving_is_an_empty_reply_not_an_error() {
    let dir = std::env::temp_dir().join("bones-persistence-test-load-before-save");
    let mut persistence = ready(&dir, false);

    assert_eq!(persistence.respond("nobody_saved_yet", &[]), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn one_sender_cannot_load_another_senders_save() {
    let dir = std::env::temp_dir().join("bones-persistence-test-isolation");
    let mut persistence = ready(&dir, false);

    persistence.handle(&envelope(
        Save::TOPIC,
        "sprite_demo",
        bones_messages::EncodeMessage::encode(&Save {
            bytes: b"sprite_demo's save",
        }),
    ));

    assert_eq!(
        persistence.respond("keyecho", &[]),
        None,
        "keyecho never saved anything and must not see sprite_demo's file"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_sender_name_that_looks_like_a_path_is_rejected_not_traversed() {
    let dir = std::env::temp_dir().join("bones-persistence-test-traversal");
    let mut persistence = ready(&dir, false);

    persistence.handle(&envelope(
        Save::TOPIC,
        "../evil",
        bones_messages::EncodeMessage::encode(&Save {
            bytes: b"should not land anywhere",
        }),
    ));

    assert!(persistence.resolve_path("../evil").is_none());
    assert_eq!(persistence.respond("../evil", &[]), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_only_mode_drops_new_saves_but_still_serves_existing_ones() {
    let dir = std::env::temp_dir().join("bones-persistence-test-read-only");
    // Seed a save while writable, then reopen read-only against the same
    // directory — read-only mode blocks new writes, not existing reads.
    {
        let mut writable = ready(&dir, false);
        writable.handle(&envelope(
            Save::TOPIC,
            "sprite_demo",
            bones_messages::EncodeMessage::encode(&Save {
                bytes: b"before read-only",
            }),
        ));
    }

    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut read_only = Persistence::new(&dir, true);
    read_only.init(&mut ctx).unwrap();

    assert_eq!(
        read_only.respond("sprite_demo", &[]),
        Some(b"before read-only".to_vec()),
        "read-only mode must still serve a save written before it was enabled"
    );

    read_only.handle(&envelope(
        Save::TOPIC,
        "sprite_demo",
        bones_messages::EncodeMessage::encode(&Save {
            bytes: b"attempted overwrite",
        }),
    ));

    assert_eq!(
        read_only.respond("sprite_demo", &[]),
        Some(b"before read-only".to_vec()),
        "a save attempted in read-only mode must not overwrite the existing file"
    );

    std::fs::remove_dir_all(&dir).ok();
}
