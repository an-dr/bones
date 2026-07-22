//! Watches loaded extensions for faults (ADR-007) or changed files
//! (design/extensions.md's Reloading state) and reacts, independent of the
//! rest of the engine.

mod tracked_extension;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bus::{Bus, Registry};
use logging::Logger;
use wasm_extensions::lifecycle;
use wasm_extensions::lifecycle::Event;

use crate::loading::{attach_extension, read_file_mtime, ENGINE_SENDER};

pub(crate) use tracked_extension::TrackedExtension;

/// How often the mtime half of `check` actually stats tracked files. The
/// fault half (an atomic load) stays every-call; at 60Hz with `check`
/// called twice a frame, stat'ing every file every time is thousands of
/// syscalls/sec for a change that happens once a minute during live
/// editing.
const MTIME_SWEEP_INTERVAL: Duration = Duration::from_millis(250);

/// Quarantines a newly-faulted extension (drops its bus registration,
/// publishes `Faulted`) or hot-swaps one whose file changed (drop old,
/// load new, publish `Reloading`/`Reloaded`). The engine, and every other
/// extension, keeps running regardless.
pub struct Supervisor {
    wasm_engine: wasmtime::Engine,
    bus: Bus,
    /// The same directory `send` (ADR-010) resolves against for every
    /// loaded extension — exposed so callers can reach an extension
    /// directly, not only observe it being quarantined.
    pub registry: Registry,
    logger: Logger,
    tracked: Vec<TrackedExtension>,
    last_mtime_sweep: Option<Instant>,
    exit_requested: Arc<AtomicBool>,
}

impl Supervisor {
    pub(crate) fn new(
        wasm_engine: wasmtime::Engine,
        bus: Bus,
        registry: Registry,
        logger: Logger,
        tracked: Vec<TrackedExtension>,
        exit_requested: Arc<AtomicBool>,
    ) -> Self {
        Self {
            wasm_engine,
            bus,
            registry,
            logger,
            tracked,
            last_mtime_sweep: None,
            exit_requested,
        }
    }

    /// Checks every tracked extension once: quarantines it if newly
    /// faulted, then — no more often than `MTIME_SWEEP_INTERVAL` — reloads
    /// it if its file changed since. A replacement that fails to load is
    /// logged and the *old* instance (or nothing, if just quarantined)
    /// keeps running.
    pub fn check(&mut self) {
        for extension in self.tracked.iter_mut() {
            if extension.quarantined || !extension.shared.is_faulted() {
                continue;
            }
            self.bus.unregister(&extension.endpoint);
            // Also drop the Registry entry -- otherwise a direct `send`
            // still resolves to this faulted Host, whose `respond` returns
            // `None` and reads back as a *successful* empty reply
            // (Registry::call), silently violating messaging.md's "no
            // request ends in silence".
            self.registry.remove(&extension.name);
            extension.quarantined = true;
            self.logger.error(
                "engine",
                &format!("'{}' faulted and was quarantined", extension.name),
            );
            lifecycle::publish(&self.bus, ENGINE_SENDER, &extension.name, Event::Faulted);
        }

        let due = match self.last_mtime_sweep {
            None => true,
            Some(last) => last.elapsed() >= MTIME_SWEEP_INTERVAL,
        };
        if !due {
            return;
        }
        self.last_mtime_sweep = Some(Instant::now());

        for extension in self.tracked.iter_mut() {
            let mtime = read_file_mtime(&extension.path);
            if mtime <= extension.mtime {
                continue;
            }

            lifecycle::publish(&self.bus, ENGINE_SENDER, &extension.name, Event::Reloading);
            match attach_extension(
                &self.wasm_engine,
                &self.bus,
                &self.registry,
                &self.logger,
                &extension.path,
                &extension.name,
                &self.exit_requested,
            ) {
                Ok((ep, shared, topics)) => {
                    if !extension.quarantined {
                        self.bus.unregister(&extension.endpoint);
                    }
                    extension.endpoint = ep;
                    extension.shared = shared;
                    extension.mtime = mtime;
                    extension.quarantined = false;
                    self.logger.info(
                        "engine",
                        &format!("reloaded '{}' (subscribed: {topics:?})", extension.name),
                    );
                    lifecycle::publish(&self.bus, ENGINE_SENDER, &extension.name, Event::Reloaded);
                }
                Err(err) => {
                    self.logger.error(
                        "engine",
                        &format!(
                            "reload of '{}' failed, keeping the running instance: {err}",
                            extension.name
                        ),
                    );
                }
            }
        }
    }
}
