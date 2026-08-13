//! `files`: lets a WASM extension read a file inside one granted directory
//! through the trusted `files` native module, since extensions have no OS/file
//! API of their own (the module-vs-extension trust split).
//!
//! - There is no typed message and no topic: a read is a direct `send`
//!   (ADR-010) to the well-known [`ENDPOINT`] name, the payload being the path
//!   to read as UTF-8 — relative to the granted root, which the extension is
//!   never told and cannot escape.
//! - The reply is the file's raw bytes, or empty when there is nothing to
//!   return: no such file, not a file, larger than the module's limit, or a
//!   path that resolved outside the root. Same ambiguity `persistence`'s load
//!   reply has, and accepted for the same reason — the caller's next move is
//!   identical either way.
//! - Bytes, not text: what a page or an extension does with the contents is its
//!   own concern, so the capability does not judge encodings.
//!
//! Absent unless the embedder grants a root (`Engine::files_root`). A read
//! capability's resource is a *specific* directory, so unlike `persistence`
//! there is nothing sensible to default to.

/// The bus endpoint name the `files` native module registers under — the
/// `send` target for a direct read call.
pub const ENDPOINT: &str = "files";
