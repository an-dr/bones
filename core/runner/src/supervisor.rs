//! Watches loaded extensions for faults (ADR-007) or changed files
//! (design/extensions.md's Reloading state) and reacts, independent of the
//! rest of the engine.

mod owned_command;
mod tracked_extension;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bus::{BudgetLimits, Bus, Registry};
use logging::Logger;
use wasm_extensions::host::DisplayInfo;
use wasm_extensions::lifecycle;
use wasm_extensions::lifecycle::Event;

use crate::loading::{attach_extension, read_file_mtime, ENGINE_SENDER};

pub(crate) use owned_command::OwnedCommand;
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
    catalog: HashMap<String, PathBuf>,
    commands: Arc<Mutex<Vec<OwnedCommand>>>,
    last_mtime_sweep: Option<Instant>,
    exit_requested: Arc<AtomicBool>,
    display_info: DisplayInfo,
    budget_limits: BudgetLimits,
}

impl Supervisor {
    #[allow(clippy::too_many_arguments)]
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
        budget_limits: BudgetLimits,
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
            budget_limits,
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
                OwnedCommand::Reload(name) => self.reload(&name),
            }
        }

        for extension in self.tracked.iter_mut() {
            if extension.quarantined
                || (!extension.shared.is_faulted() && !extension.budget.has_exceeded())
            {
                continue;
            }
            let drops = extension.budget.get_drop_counters();
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
                &format!(
                    "'{}' faulted and was quarantined (dropped inbound={}, publishes={})",
                    extension.name, drops.inbound, drops.publishes
                ),
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
                self.budget_limits,
            ) {
                Ok((ep, shared, budget, topics)) => {
                    if !extension.quarantined {
                        self.bus.unregister(&extension.endpoint);
                    }
                    extension.endpoint = ep;
                    extension.shared = shared;
                    extension.budget = budget;
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

    /// Stops every running extension through the same orderly lifecycle
    /// boundary as a runtime unload.
    ///
    /// Shutdown-hook publishes stay queued ahead of each `Stopped` event.
    pub fn shutdown_all(&mut self) {
        let running: Vec<String> = self
            .tracked
            .iter()
            .filter(|extension| !extension.quarantined)
            .map(|extension| extension.name.clone())
            .collect();
        for name in running {
            self.unload(&name, true);
        }
    }

    /// Activates a catalog entry.
    ///
    /// Idempotent: an already-running extension succeeds without replacing
    /// its instance.
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
            self.logger.error(
                "engine",
                &format!("extension '{name}' is not in the catalog"),
            );
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
            self.budget_limits,
        ) {
            Ok((endpoint, shared, budget, topics)) => {
                self.tracked.retain(|extension| extension.name != name);
                self.tracked.push(TrackedExtension {
                    name: name.to_string(),
                    path: path.clone(),
                    mtime: read_file_mtime(&path),
                    endpoint,
                    shared,
                    budget,
                    quarantined: false,
                });
                self.logger.info(
                    "engine",
                    &format!(
                        "loaded '{name}' from {} (subscribed: {topics:?})",
                        path.display()
                    ),
                );
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, success_event);
            }
            Err(err) => {
                self.logger.error(
                    "engine",
                    &format!("failed to load {}: {err}", path.display()),
                );
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Faulted);
            }
        }
    }

    /// Replaces a running extension transactionally.
    ///
    /// The replacement attaches before the old endpoint is shut down. A
    /// missing or invalid replacement is logged and leaves the current
    /// instance registered and running.
    fn reload(&mut self, name: &str) {
        lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Reloading);

        let Some(path) = self.catalog.get(name).cloned() else {
            self.logger.error(
                "engine",
                &format!("reload of '{name}' failed: extension is not in the catalog"),
            );
            return;
        };
        let Some(index) = self
            .tracked
            .iter()
            .position(|extension| extension.name == name && !extension.quarantined)
        else {
            self.load(name, Event::Reloaded);
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
            self.budget_limits,
        ) {
            Ok((endpoint, shared, budget, topics)) => {
                let extension = &mut self.tracked[index];
                if let Err(err) = extension.shared.shutdown() {
                    self.logger
                        .error("engine", &format!("shutdown of '{name}' failed: {err}"));
                }
                self.bus.unregister(&extension.endpoint);
                extension.path = path.clone();
                extension.mtime = read_file_mtime(&path);
                extension.endpoint = endpoint;
                extension.shared = shared;
                extension.budget = budget;
                extension.quarantined = false;
                self.logger.info(
                    "engine",
                    &format!(
                        "reloaded '{name}' from {} (subscribed: {topics:?})",
                        path.display()
                    ),
                );
                lifecycle::publish(&self.bus, ENGINE_SENDER, name, Event::Reloaded);
            }
            Err(err) => {
                self.logger.error(
                    "engine",
                    &format!("reload of '{name}' failed, keeping the running instance: {err}"),
                );
            }
        }
    }

    /// Deactivates an extension.
    ///
    /// Idempotent: an absent extension succeeds and, when requested,
    /// publishes `Stopped`.
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
