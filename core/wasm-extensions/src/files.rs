//! Lets a sandboxed WASM extension — which has no OS/file API of its own, by
//! design (the module-vs-extension trust split) — read files inside one
//! directory the embedder granted, mediated through this trusted native module
//! and the bus instead of a filesystem capability grant per extension.
//!
//! - A read is a direct `send` (ADR-010, `Module::respond`) to the well-known
//!   `files` endpoint, the payload being a path relative to the granted root;
//!   the reply is the file's bytes, or empty when there is nothing to return.
//!   See `bones_messages::files` for the protocol itself.
//! - Opt-in, unlike `persistence`: the resource here is one specific directory,
//!   which only the embedder can name (`Engine::files_root`). Without a root
//!   the module is not registered at all, so the endpoint simply does not exist
//!   and every read fails the way an unknown endpoint does.
//! - Containment is resolved, not trusted: both the root and the target are
//!   canonicalised, and a target that does not sit under the root is refused.
//!   That holds for a path escaping through `..`, through an absolute path, and
//!   through a symlink pointing out of the tree.
//! - A size limit keeps one read from pulling an arbitrarily large file across
//!   the bus in a single reply.
//!
//! No logger, for the same reason `persistence` has none: the module must work
//! whether it is constructed by `Engine::build` or by an embedder's own
//! `.module(...)` call. A refused or failed read is a silent empty reply.

use std::path::{Path, PathBuf};

use bones_messages::files::ENDPOINT;
use bus::{Envelope, Handler, Module, ModuleContext};

/// Largest file served in one reply, when the embedder states no other limit.
pub const DEFAULT_MAX_BYTES: u64 = 4 * 1024 * 1024;

pub struct Files {
    root: PathBuf,
    max_bytes: u64,
}

impl Files {
    /// `root` is the only directory reads may reach. Relative paths resolve the
    /// same way `extensions_dir` and `saves_dir` do, against the running
    /// executable rather than the process's cwd — `Engine::build` applies that
    /// before constructing this.
    pub fn new(root: impl Into<PathBuf>, max_bytes: u64) -> Self {
        Self {
            root: root.into(),
            max_bytes,
        }
    }

    /// Resolves `request` inside the root, or `None` when it does not belong to
    /// it or cannot be read.
    fn resolve(&self, request: &str) -> Option<PathBuf> {
        if request.is_empty() {
            return None;
        }
        // Canonicalising both sides is what makes the check meaningful: it
        // resolves `..`, symlinks, and (on Windows) short names before the
        // comparison, so no textual trick reaches outside the root.
        let root = self.root.canonicalize().ok()?;
        let target = root.join(request).canonicalize().ok()?;
        target.starts_with(&root).then_some(target)
    }

    /// Reads a resolved path, subject to the size limit.
    fn read(&self, path: &Path) -> Option<Vec<u8>> {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() || metadata.len() > self.max_bytes {
            return None;
        }
        std::fs::read(path).ok()
    }
}

/// No topics: reading needs a reply, so the capability is only ever a direct
/// `send`, and there is nothing to subscribe to.
impl Handler for Files {
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for Files {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
        Ok(())
    }

    /// Serves one read. `sender` is unused: every extension granted the root
    /// sees the same tree, unlike `persistence`, where the sender decides which
    /// file is even addressed.
    fn respond(&mut self, _sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        let request = std::str::from_utf8(payload).ok()?;
        self.read(&self.resolve(request)?)
    }
}

#[cfg(test)]
mod tests;
