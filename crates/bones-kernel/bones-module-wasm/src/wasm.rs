use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bones_kernel_infra::bus::{
    BudgetLimits, Bus, Envelope, Handler, Module, ModuleContext, Registry,
};
use bones_kernel_infra::display::DisplayInfo;
use bones_kernel_infra::exit::ExitRequest;
use bones_kernel_infra::logging::Logger;
use bones_messages::extension_control::Command;

use crate::host::{new_engine, ExtensionTimeouts};
use crate::lifecycle::{self, Event};
use crate::loading::{
    attach_extension, derive_extension_name, find_wasm_files, read_file_mtime, ENGINE_SENDER,
};
use crate::permissions::Permissions;
use crate::supervisor::{OwnedCommand, Supervisor, TrackedExtension};

/// The bus endpoint the module answers on: runtime commands arrive here on
/// `core/extensions/*`. Guests address runtime commands to this name.
const ENDPOINT: &str = "extension-manager";

/// The extension host module.
///
/// Configure it, then register it after every module an extension may call
/// during its own `init` — `Kernel`'s builder does both when the `wasm`
/// feature is on. Its `init` loads the startup extensions; its frame hooks
/// apply hot reloads and quarantine faults; its `begin_shutdown` stops every
/// extension while the other modules can still hear from them.
pub struct Wasm {
    logger: Logger,
    extensions_dir: Option<PathBuf>,
    catalog_entries: Vec<(String, PathBuf)>,
    startup: Vec<String>,
    required: Vec<String>,
    controller: Option<String>,
    budget: BudgetLimits,
    timeouts: ExtensionTimeouts,
    commands: Arc<Mutex<Vec<OwnedCommand>>>,
    supervisor: Option<Supervisor>,
    permissions: HashMap<String, Permissions>,
}

impl Wasm {
    /// A host that loads nothing until configured.
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            extensions_dir: None,
            catalog_entries: Vec::new(),
            startup: Vec::new(),
            required: Vec::new(),
            controller: None,
            budget: BudgetLimits::default(),
            timeouts: ExtensionTimeouts::default(),
            commands: Arc::new(Mutex::new(Vec::new())),
            supervisor: None,
            permissions: HashMap::new(),
        }
    }

    /// Catalogs every `.wasm` file beneath `dir`, recursively; the file stem
    /// is the extension's name.
    pub fn extensions_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.extensions_dir = Some(dir.into());
        self
    }

    /// Catalogs one extension under an explicit name.
    pub fn catalog_extension(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.catalog_entries.push((name.into(), path.into()));
        self
    }

    /// Loads only these at startup; the rest of the catalog stays available
    /// to `core/extensions/load`. Empty, the default, loads everything.
    pub fn startup_extensions(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.startup = names.into_iter().collect();
        self
    }

    /// Loads these at startup like [`startup_extensions`](Self::startup_extensions),
    /// and fails `init` -- so the kernel's `build` -- when any of them is not
    /// in the catalog or does not load. Without it a failed startup load is
    /// only logged, and an app whose window that extension drives shows
    /// nothing at all.
    pub fn required_extensions(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.required = names.into_iter().collect();
        self
    }

    /// The one sender allowed to load, unload, and reload extensions at
    /// runtime. Commands from anyone else are rejected and logged.
    pub fn controller(mut self, name: impl Into<String>) -> Self {
        self.controller = Some(name.into());
        self
    }

    /// The queue and publish budgets every extension endpoint gets.
    pub fn budget(mut self, budget: BudgetLimits) -> Self {
        self.budget = budget;
        self
    }

    /// The wall-clock budgets for loading and for each call.
    pub fn timeouts(mut self, timeouts: ExtensionTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Denies the extension called `name` the topics and endpoints in
    /// `permissions` (ADR-042), for every instance loaded under that name.
    pub fn permissions(mut self, name: impl Into<String>, permissions: Permissions) -> Self {
        self.permissions.insert(name.into(), permissions);
        self
    }

    fn supervise(&mut self) {
        if let Some(supervisor) = &mut self.supervisor {
            supervisor.check();
        }
    }
}

impl Handler for Wasm {
    fn handle(&mut self, envelope: &Envelope) {
        if self.controller.as_deref() != Some(envelope.sender.as_str()) {
            self.logger.error(
                "engine",
                &format!(
                    "rejected runtime extension command from '{}' on '{}'",
                    envelope.sender, envelope.topic
                ),
            );
            return;
        }
        match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => self
                .commands
                .lock()
                .unwrap()
                .push(OwnedCommand::from(command)),
            Ok(None) => self.logger.warn(
                "engine",
                &format!(
                    "ignored unknown extension command topic '{}'",
                    envelope.topic
                ),
            ),
            Err(err) => self.logger.warn(
                "engine",
                &format!(
                    "could not decode extension command from '{}' on '{}': {err}",
                    envelope.sender, envelope.topic
                ),
            ),
        }
    }
}

impl Module for Wasm {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        let bus = ctx
            .get_service::<Bus>()
            .cloned()
            .ok_or("wasm needs the bus service")?;
        let registry = ctx
            .get_service::<Registry>()
            .cloned()
            .ok_or("wasm needs the registry service")?;
        let exit_requested = ctx
            .get_service::<ExitRequest>()
            .cloned()
            .unwrap_or_default();
        let display_info = ctx
            .get_service::<DisplayInfo>()
            .cloned()
            .unwrap_or_default();
        let wasm_engine = new_engine().map_err(|err| err.to_string())?;

        let mut entries = Vec::new();
        if let Some(dir) = &self.extensions_dir {
            for path in find_wasm_files(dir) {
                entries.push((derive_extension_name(&path), path));
            }
        }
        entries.append(&mut self.catalog_entries);

        for name in &self.required {
            if !self.startup.is_empty() && !self.startup.contains(name) {
                self.startup.push(name.clone());
            }
        }
        let mut catalog = HashMap::new();
        let mut tracked = Vec::new();
        let mut failed = Vec::new();
        for (name, path) in entries {
            if catalog.contains_key(&name) {
                self.logger.error(
                    "engine",
                    &format!(
                        "skipping {}: an extension named '{name}' is already cataloged",
                        path.display()
                    ),
                );
                continue;
            }
            catalog.insert(name.clone(), path.clone());
            if !self.startup.is_empty() && !self.startup.contains(&name) {
                continue;
            }
            match attach_extension(
                &wasm_engine,
                &bus,
                &registry,
                &self.logger,
                &path,
                &name,
                &exit_requested,
                &display_info,
                self.budget,
                self.timeouts,
                &self.permissions.get(&name).cloned().unwrap_or_default(),
            ) {
                Ok((endpoint, shared, budget, topics)) => {
                    self.logger.info(
                        "engine",
                        &format!(
                            "loaded '{name}' from {} (subscribed: {topics:?})",
                            path.display()
                        ),
                    );
                    lifecycle::publish(&bus, ENGINE_SENDER, &name, Event::Loaded);
                    tracked.push(TrackedExtension {
                        name,
                        mtime: read_file_mtime(&path),
                        path,
                        endpoint,
                        shared,
                        budget,
                        quarantined: false,
                    });
                }
                Err(err) => {
                    self.logger.error(
                        "engine",
                        &format!("failed to load {}: {err}", path.display()),
                    );
                    lifecycle::publish(&bus, ENGINE_SENDER, &name, Event::Faulted);
                    if self.required.contains(&name) {
                        failed.push(format!("required extension '{name}' failed to load: {err}"));
                    }
                }
            }
        }
        for name in &self.startup {
            if !catalog.contains_key(name) {
                self.logger.error(
                    "engine",
                    &format!("startup extension '{name}' is not in the catalog"),
                );
            }
        }
        for name in &self.required {
            if !catalog.contains_key(name) {
                failed.push(format!("required extension '{name}' is not in the catalog"));
            }
        }

        ctx.subscribe("core/extensions/*");
        self.supervisor = Some(Supervisor::new(
            wasm_engine,
            bus,
            registry,
            self.logger.clone(),
            tracked,
            catalog,
            self.commands.clone(),
            exit_requested,
            display_info,
            self.budget,
            self.timeouts,
            self.permissions.clone(),
        ));
        if !failed.is_empty() {
            // Stops what did load, since the engine will not run to stop it.
            if let Some(supervisor) = &mut self.supervisor {
                supervisor.shutdown_all();
            }
            return Err(failed.join("; "));
        }
        Ok(())
    }

    fn before_tick(&mut self) {
        self.supervise();
    }

    fn after_tick(&mut self) {
        self.supervise();
    }

    fn begin_shutdown(&mut self) {
        if let Some(supervisor) = &mut self.supervisor {
            supervisor.check();
            supervisor.shutdown_all();
        }
    }
}
