//! Extension discovery and the load-register-subscribe-insert ceremony
//! shared by initial discovery (`Engine::build`) and hot reload
//! (`Supervisor::check`) — one function so the two paths can't drift out
//! of lockstep.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bus::{Bus, Endpoint, Envelope, Handler, Registry, Respond};
use logging::Logger;
use wasm_extensions::host::Host;

/// Publishes lifecycle events as this component (design/extensions.md).
pub(crate) const ENGINE_SENDER: &str = "engine";

/// One extension `Host`, shared between its `Bus` registration (pub/sub
/// delivery), its `Registry` registration (direct send, ADR-010), and the
/// `Supervisor` (which needs `Host::is_faulted` after every call to know
/// when to quarantine it) — all three need the same instance, not
/// independent copies, so state stays consistent across all of them.
#[derive(Clone)]
pub(crate) struct SharedHost(Arc<Mutex<Host>>);

impl Handler for SharedHost {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

impl Respond for SharedHost {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().respond(sender, payload)
    }
}

impl SharedHost {
    pub(crate) fn is_faulted(&self) -> bool {
        self.0.lock().unwrap().is_faulted()
    }
}

/// Loads `path` as an extension named `name`, registers it on `bus`
/// (pub/sub) and `registry` (direct send, ADR-010), and subscribes it to
/// whatever topics it requested via `subscribe` during `init`.
pub(crate) fn attach_extension(
    wasm_engine: &wasmtime::Engine,
    bus: &Bus,
    registry: &Registry,
    logger: &Logger,
    path: &Path,
    name: &str,
) -> wasmtime::Result<(Endpoint, SharedHost, Vec<String>)> {
    let mut extension = Host::load(wasm_engine, &path.to_string_lossy(), name, bus.clone(), registry.clone(), logger.clone())?;
    let topics = extension.requested_topics();
    let shared = SharedHost(Arc::new(Mutex::new(extension)));

    let ep = bus.register(name.to_string(), shared.clone());
    for topic in &topics {
        ep.subscribe(topic);
    }
    registry.insert(name.to_string(), Arc::new(shared.clone()));

    Ok((ep, shared, topics))
}

pub(crate) fn read_file_mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// `.wasm` files directly inside `dir`, sorted for deterministic load order.
/// A missing directory is "no extensions," not an error.
pub(crate) fn find_wasm_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    files.sort();
    files
}

pub(crate) fn derive_extension_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// True the first time `name` is seen (and records it); false on a repeat.
/// A single real directory listing can never actually repeat a stem, but a
/// future multi-directory `extensions_dir` could — this is what makes
/// extensions.md's "the host rejects duplicates at load" claim true.
pub(crate) fn is_first_occurrence(seen: &mut std::collections::HashSet<String>, name: &str) -> bool {
    seen.insert(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Built by extensions/hello/build.ps1 (see its README).
    const HELLO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/hello/target/wasm32-wasip2/release"
    );

    #[test]
    fn find_wasm_files_finds_only_wasm_extensions_sorted() {
        let files = find_wasm_files(Path::new(HELLO_DIR));
        assert!(
            files.iter().all(|f| f.extension().unwrap() == "wasm"),
            "expected only .wasm files, got {files:?}"
        );
        assert!(
            files.iter().any(|f| f.file_stem().unwrap() == "hello"),
            "expected hello.wasm in {files:?} — run extensions/hello/build.ps1 first"
        );
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted);
    }

    #[test]
    fn find_wasm_files_on_a_missing_directory_is_empty_not_an_error() {
        assert_eq!(find_wasm_files(Path::new("no/such/directory")), Vec::<PathBuf>::new());
    }

    #[test]
    fn derive_extension_name_is_the_file_stem() {
        assert_eq!(derive_extension_name(Path::new("/a/b/hello.wasm")), "hello");
    }

    #[test]
    fn is_first_occurrence_accepts_a_name_once_and_rejects_a_repeat() {
        let mut seen = std::collections::HashSet::new();
        assert!(is_first_occurrence(&mut seen, "hello"));
        assert!(!is_first_occurrence(&mut seen, "hello"));
        assert!(is_first_occurrence(&mut seen, "keyecho"));
    }
}
