//! Extension discovery and the load-register-subscribe-insert ceremony
//! shared by initial discovery (`Engine::build`) and hot reload
//! (`Supervisor::check`) — one function so the two paths can't drift out
//! of lockstep.

mod shared_host;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bus::{BudgetLimits, Bus, Endpoint, EndpointBudget, Registry};
use logging::Logger;
use wasm_extensions::host::{DisplayInfo, Host};

pub(crate) use shared_host::SharedHost;

/// Publishes lifecycle events as this component (design/extensions.md).
pub(crate) const ENGINE_SENDER: &str = "engine";

/// Loads `path` as an extension named `name`, registers it on `bus`
/// (pub/sub) and `registry` (direct send, ADR-010), and subscribes it to
/// whatever topics it requested via `subscribe` during `init`. `exit_requested`
/// is shared with every extension this way — any one of them calling
/// `request-exit` sets the same flag the caller's own run loop reads.
#[allow(clippy::too_many_arguments)]
pub(crate) fn attach_extension(
    wasm_engine: &wasmtime::Engine,
    bus: &Bus,
    registry: &Registry,
    logger: &Logger,
    path: &Path,
    name: &str,
    exit_requested: &Arc<AtomicBool>,
    display_info: &DisplayInfo,
    budget_limits: BudgetLimits,
) -> wasmtime::Result<(Endpoint, SharedHost, EndpointBudget, Vec<String>)> {
    let budget = EndpointBudget::new(budget_limits);
    let mut extension = Host::load(
        wasm_engine,
        &path.to_string_lossy(),
        name,
        bus.clone(),
        registry.clone(),
        logger.clone(),
        exit_requested.clone(),
        display_info.clone(),
        budget.clone(),
    )?;
    let topics = extension.requested_topics();
    let shared = SharedHost(Arc::new(Mutex::new(extension)));

    let ep = bus.register_with_budget(name.to_string(), shared.clone(), budget.clone());
    for topic in &topics {
        ep.subscribe(topic);
    }
    registry.insert(name.to_string(), Arc::new(shared.clone()));

    Ok((ep, shared, budget, topics))
}

pub(crate) fn read_file_mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// `.wasm` files beneath `dir`, sorted for deterministic load order.
///
/// - A missing directory is "no extensions," not an error.
/// - Symbolic links are skipped, so a linked directory cycle cannot recurse.
pub(crate) fn find_wasm_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.filter_map(|entry| entry.ok()) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            files.extend(find_wasm_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "wasm") {
            files.push(path);
        }
    }
    files.sort();
    files
}

pub(crate) fn derive_extension_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests;
