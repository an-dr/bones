//! Extension discovery and the load-register-subscribe-insert ceremony
//! shared by initial discovery (`Engine::build`) and hot reload
//! (`Supervisor::check`) — one function so the two paths can't drift out
//! of lockstep.

mod shared_host;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bus::{Bus, Endpoint, Registry};
use logging::Logger;
use wasm_extensions::host::Host;

pub(crate) use shared_host::SharedHost;

/// Publishes lifecycle events as this component (design/extensions.md).
pub(crate) const ENGINE_SENDER: &str = "engine";

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
    let mut extension = Host::load(
        wasm_engine,
        &path.to_string_lossy(),
        name,
        bus.clone(),
        registry.clone(),
        logger.clone(),
    )?;
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
pub(crate) fn is_first_occurrence(
    seen: &mut std::collections::HashSet<String>,
    name: &str,
) -> bool {
    seen.insert(name.to_string())
}

#[cfg(test)]
mod tests;
