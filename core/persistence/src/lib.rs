//! Persistence module (design/modules.md, ADR-011, ADR-018): lets a
//! sandboxed WASM extension — which has no OS/file API of its own, by
//! design (the module-vs-extension trust split) — save and restore state
//! across a reload, mediated through a trusted native module and the bus
//! instead of a filesystem capability grant per extension.
//!
//! Save (`persistence/save`, pub/sub) writes `<sender>.bin`; load is a
//! direct `send` (ADR-010, `Module::respond`) to the well-known
//! `persistence` endpoint with an empty payload, replying with the raw
//! saved bytes or an empty reply if there's nothing saved — see
//! `bones_messages::persistence`'s doc comment for why that's an accepted
//! ambiguity for now. No logger (same limitation `core/audio` already
//! documents: the generic `.module(...)` construction path has no access
//! to `Engine`'s internal `Logger`) — I/O failures are silent no-ops.

use std::path::PathBuf;

use bones_messages::persistence::Save;
use bones_messages::{DecodeMessage, Message};
use bus::{Envelope, Handler, Module, ModuleContext};

pub struct Persistence {
    dir: PathBuf,
}

impl Persistence {
    /// `dir` is where `<sender>.bin` save files live, one per extension
    /// name — relative to the process's cwd if not absolute, the same
    /// convention `extensions/` and `dist/` already use. Explicit rather
    /// than a hardcoded default so tests can point it at a scratch
    /// directory instead of the real one.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `<dir>/<sender>.bin`, or `None` if `sender` isn't a plain name —
    /// `sender` is host-stamped from the caller's own registered endpoint
    /// name, never guest-suppliable (`core/host`'s `publish`/`send`
    /// implementations always use their own fixed name), so this isn't a
    /// defense against a malicious extension. It is a defense against a
    /// misconfigured one: a module or extension registered under an
    /// unusual name (a path-like string) must not be able to save outside
    /// its own directory even by accident.
    fn resolve_path(&self, sender: &str) -> Option<PathBuf> {
        if sender.is_empty() || sender.contains(['/', '\\']) || sender.contains("..") {
            return None;
        }
        Some(self.dir.join(format!("{sender}.bin")))
    }
}

impl Handler for Persistence {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Save::TOPIC {
            return;
        }
        let Ok(save) = Save::decode(&envelope.payload) else {
            return;
        };
        if let Some(path) = self.resolve_path(&envelope.sender) {
            let _ = std::fs::write(path, save.bytes);
        }
    }
}

impl Module for Persistence {
    fn name(&self) -> &str {
        "persistence"
    }

    /// Ensures the save directory exists. Errors (e.g. no write
    /// permission) fail the module the same way renderer's missing
    /// `window-surface` or audio's missing device do — a caller/
    /// environment mistake, not a runtime condition to silently degrade
    /// from.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("persistence/*");
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("creating save directory {}: {e}", self.dir.display()))?;
        Ok(())
    }

    /// Loads `sender`'s own save file — a direct call, not pub/sub,
    /// because unlike save, load actually needs a reply. `payload` is
    /// ignored (nothing to configure about a load beyond "whose").
    fn respond(&mut self, sender: &str, _payload: &[u8]) -> Option<Vec<u8>> {
        std::fs::read(self.resolve_path(sender)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::ServiceRegistry;

    #[test]
    fn init_creates_the_save_directory_and_subscribes_persistence_topics() {
        let dir = std::env::temp_dir().join("bones-persistence-test-init");
        std::fs::remove_dir_all(&dir).ok();
        assert!(!dir.exists(), "test setup: directory must not pre-exist");

        let mut registry = ServiceRegistry::new();
        let mut ctx = ModuleContext::new(&mut registry);
        let mut persistence = Persistence::new(&dir);

        persistence.init(&mut ctx).expect("creating a scratch directory should succeed");

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
        let mut persistence = Persistence::new(&dir);

        persistence.init(&mut ctx).expect("an already-existing directory is not an error");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn name_is_persistence() {
        assert_eq!(Persistence::new("saves").name(), "persistence");
    }

    fn ready(dir: &std::path::Path) -> Persistence {
        std::fs::remove_dir_all(dir).ok();
        let mut registry = ServiceRegistry::new();
        let mut ctx = ModuleContext::new(&mut registry);
        let mut persistence = Persistence::new(dir);
        persistence.init(&mut ctx).unwrap();
        persistence
    }

    #[test]
    fn save_then_load_round_trips_through_a_real_file() {
        let dir = std::env::temp_dir().join("bones-persistence-test-round-trip");
        let mut persistence = ready(&dir);

        persistence.handle(&Envelope {
            topic: Save::TOPIC.to_string(),
            sender: "sprite_demo".to_string(),
            correlation: None,
            payload: bones_messages::EncodeMessage::encode(&Save { bytes: b"level=3;hp=42" }),
        });

        let loaded = persistence.respond("sprite_demo", &[]);

        assert_eq!(loaded, Some(b"level=3;hp=42".to_vec()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loading_before_ever_saving_is_an_empty_reply_not_an_error() {
        let dir = std::env::temp_dir().join("bones-persistence-test-load-before-save");
        let mut persistence = ready(&dir);

        assert_eq!(persistence.respond("nobody_saved_yet", &[]), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn one_sender_cannot_load_another_senders_save() {
        let dir = std::env::temp_dir().join("bones-persistence-test-isolation");
        let mut persistence = ready(&dir);

        persistence.handle(&Envelope {
            topic: Save::TOPIC.to_string(),
            sender: "sprite_demo".to_string(),
            correlation: None,
            payload: bones_messages::EncodeMessage::encode(&Save { bytes: b"sprite_demo's save" }),
        });

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
        let mut persistence = ready(&dir);

        persistence.handle(&Envelope {
            topic: Save::TOPIC.to_string(),
            sender: "../evil".to_string(),
            correlation: None,
            payload: bones_messages::EncodeMessage::encode(&Save { bytes: b"should not land anywhere" }),
        });

        assert!(persistence.resolve_path("../evil").is_none());
        assert_eq!(persistence.respond("../evil", &[]), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
