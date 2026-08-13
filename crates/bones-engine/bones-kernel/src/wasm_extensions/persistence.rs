//! Lets a sandboxed WASM extension — which has no OS/file API of its own,
//! by design (the module-vs-extension trust split) — save and restore
//! state across a reload, mediated through this trusted native module and
//! the bus instead of a filesystem capability grant per extension.
//!
//! Save (`persistence/save`, pub/sub) writes `<sender>.bin`; load is a
//! direct `send` (ADR-010, `Module::respond`) to the well-known
//! `persistence` endpoint with an empty payload, replying with the raw
//! saved bytes or an empty reply if there's nothing saved — see
//! `bones_messages::persistence`'s doc comment for why that's an accepted
//! ambiguity for now.
//!
//! Unconditional, unlike `audio`/`renderer`/`ui`: `Engine::build` always
//! registers this, no `bones.toml` toggle to skip it — disabling it saves
//! no dependency weight (`std::fs` only) and its resource, a writable
//! directory, exists on every environment bones targets. What *is*
//! configurable is read-only mode (`Engine::read_only_persistence`):
//! extensions can still load previously-saved state, but new saves are
//! silently dropped — a policy choice (e.g. an audited/locked-down
//! extension sandbox), not a resource one.
//!
//! No logger: the module needs to work whether it's constructed through
//! `Engine::build`'s own unconditional wiring or (in principle) a
//! `.module(...)` call, and the latter path has no access to `Engine`'s
//! internal `Logger` — see the `TODO` on `Engine::module`. I/O failures
//! are silent no-ops.

use std::path::PathBuf;

use crate::bus::{Envelope, Handler, Module, ModuleContext};
use bones_messages::persistence::Save;
use bones_messages::{DecodeMessage, Message};

pub struct Persistence {
    dir: PathBuf,
    read_only: bool,
}

impl Persistence {
    /// `dir` is where `<sender>.bin` save files live, one per extension
    /// name — relative to the process's cwd if not absolute, the same
    /// convention `extensions/` and `dist/` already use. Explicit rather
    /// than a hardcoded default so tests can point it at a scratch
    /// directory instead of the real one.
    pub fn new(dir: impl Into<PathBuf>, read_only: bool) -> Self {
        Self {
            dir: dir.into(),
            read_only,
        }
    }

    /// `<dir>/<sender>.bin`, or `None` if `sender` isn't a plain name —
    /// `sender` is host-stamped from the caller's own registered endpoint
    /// name, never guest-suppliable (this crate's `host` module's `publish`/`send`
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
        if envelope.topic != Save::TOPIC || self.read_only {
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

    /// Ensures the save directory exists — even in read-only mode, since a
    /// missing directory is itself the kind of environment mistake that
    /// should fail loudly (same stance as renderer's missing
    /// `window-surface` or audio's missing device), not something
    /// read-only mode should paper over.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("persistence/*");
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("creating save directory {}: {e}", self.dir.display()))?;
        Ok(())
    }

    /// Loads `sender`'s own save file — a direct call, not pub/sub,
    /// because unlike save, load actually needs a reply. `payload` is
    /// ignored (nothing to configure about a load beyond "whose"). Always
    /// allowed, even in read-only mode: that mode blocks new writes, not
    /// reading what's already there.
    fn respond(&mut self, sender: &str, _payload: &[u8]) -> Option<Vec<u8>> {
        std::fs::read(self.resolve_path(sender)?).ok()
    }
}

#[cfg(test)]
mod tests;
