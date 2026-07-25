//! Watches loaded extensions for faults (ADR-007) or changed files
//! (design/extensions.md's Reloading state) and reacts, independent of the
//! rest of the engine.

mod tracked_extension;
mod owned_command;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bus::{Bus, Registry};
use logging::Logger;
use wasm_extensions::host::DisplayInfo;
use wasm_extensions::lifecycle;
use wasm_extensions::lifecycle::Event;

use crate::loading::{attach_extension, read_file_mtime, ENGINE_SENDER};

pub(crate) use tracked_extension::TrackedExtension;
pub(crate) use owned_command::OwnedCommand;

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
    catalog: HashMap<String, PathBuf>,
    commands: Arc<Mutex<Vec<OwnedCommand>>>,
    last_mtime_sweep: Option<Instant>,
    exit_requested: Arc<AtomicBool>,
    display_info: DisplayInfo,
}

impl Supervisor {
    pub(crate) fn new(
        wasm_engine: wasmtime::Engine,
        bus: Bus,
        registry: Registry,
        logger: Logger,
        tracked: Vec<TrackedExtension>,
        catalog: HashMap<String, PathBuf>,
        commands: Arc<Mutex<Vec<OwnedCommand>>>,
        exit_requested: Arc<AtomicBool>,
        display_info: DisplayInfo,
    ) -> Self {
        Self {
            wasm_engine,
            bus,
            registry,
            logger,
            tracked,
            catalog,
            commands,
            last_mtime_sweep: None,
            exit_requested,
            display_info,
        }
    }

    /// Checks every tracked extension once: quarantines it if newly
    /// faulted, then — no more often than `MTIME_SWEEP_INTERVAL` — reloads
    /// it if its file changed since. A replacement that fails to load is
    /// logged and the *old* instance (or nothing, if just quarantined)
    /// keeps running.
    pub fn check(&mut self) {
        let commands = std::mem::take(&mut *self.commands.lock().unwrap());
        for command in commands {
            match command {
                OwnedCommand::Load(name) => self.load(&name, Event::Loaded),
                OwnedCommand::Unload(name) => self.unload(&name, true),
                OwnedCommand::Reload(name) => {
                    lifecycle::publish(&self.bus, ENGINE_SENDER, &name, Event::Reloading);
                    self.unload(&name, false);
                    self.load(&name, Event::Reloaded);
                }
            }
        }

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
                &self.display_info,
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

    fn load(&mut self, name: &str, success_event: Event) {
        if self
            .tracked
            .iter()
            .any(|extension| extension.name == name && !extension.quarantined)
        {
            lifecycle::publish(&self.bus, ENGINE_SENDER, name, success_event);
            return;
        }
        let Some(path) = self.catalog.get(name).cloned() else {
            self.logger
                .error("engine", &format!("extension '{name}' is not in the catalog"));
            lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Faulted);
            return;
        };
        match attach_extension(
            &self.wasm_engine,
            &self.bus,
            &self.registry,
            &self.logger,
            &path,
            name,
            &self.exit_requested,
            &self.display_info,
        ) {
            Ok((endpoint, shared, topics)) => {
                self.tracked.retain(|extension| extension.name != name);
                self.tracked.push(TrackedExtension {
                    name: name.to_string(),
                    path: path.clone(),
                    mtime: read_file_mtime(&path),
                    endpoint,
                    shared,
                    quarantined: false,
                });
                self.logger.info(
                    "engine",
                    &format!("loaded '{name}' from {} (subscribed: {topics:?})", path.display()),
                );
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, success_event);
            }
            Err(err) => {
                self.logger
                    .error("engine", &format!("failed to load {}: {err}", path.display()));
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Faulted);
            }
        }
    }

    fn unload(&mut self, name: &str, publish_stopped: bool) {
        let Some(index) = self
            .tracked
            .iter()
            .position(|extension| extension.name == name)
        else {
            if publish_stopped {
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Stopped);
            }
            return;
        };
        let extension = self.tracked.remove(index);
        if !extension.quarantined {
            if let Err(err) = extension.shared.shutdown() {
                self.logger
                    .error("engine", &format!("shutdown of '{name}' failed: {err}"));
            }
            self.bus.unregister(&extension.endpoint);
            self.registry.remove(name);
        }
        if publish_stopped {
            lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Stopped);
        }
    }
}
