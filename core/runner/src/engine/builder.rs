//! The public builder API (design/modules.md, ADR-017): discovers WASM
//! extensions and runs them, plus `.module(...)` for injecting custom
//! native modules. TODO: `bones::Engine` in the design sketch — lives here
//! as `runner::Engine` until a top-level facade crate exists to re-export
//! it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "presentation")]
use bones_messages::Message;
#[cfg(feature = "presentation")]
use bus::ModuleContext;
use bus::{BudgetLimits, Module, Registry, ServiceRegistry};
use logging::Logger;
#[cfg(feature = "presentation")]
use renderer::Renderer;
#[cfg(feature = "presentation")]
use ui::Ui;
use wasm_extensions::host::DisplayInfo;
use wasm_extensions::lifecycle;
use wasm_extensions::lifecycle::Event;
use wasm_extensions::persistence::Persistence;

use crate::loading::{
    attach_extension, derive_extension_name, find_wasm_files, read_file_mtime, ENGINE_SENDER,
};
use crate::supervisor::TrackedExtension;
use crate::Runner;
use crate::Supervisor;

use super::built_engine::{run_shutdown, BuiltEngine};
use super::register_module::register_module;
#[cfg(feature = "presentation")]
use super::shared::Shared;

const DEFAULT_TICK_HZ: f64 = 60.0;
#[cfg(feature = "presentation")]
const GFX_TOPICS: &str = "gfx/*";

/// A relative `extensions_dir`/`saves_dir` resolves against the running
/// executable's own directory, not the process's current working
/// directory -- so a shipped build behaves the same whether launched by
/// double-click, shortcut, or from an arbitrary shell. Absolute paths
/// pass through unchanged.
fn resolve_relative_to_exe(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    match exe_dir() {
        Some(dir) => dir.join(path),
        None => path,
    }
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

pub struct Engine {
    extensions_dir: Option<PathBuf>,
    catalog_extensions: Vec<(String, PathBuf)>,
    startup_extensions: Vec<String>,
    extension_controller: Option<String>,
    logger: Logger,
    tick_hz: f64,
    #[cfg(feature = "presentation")]
    window: Option<(String, u32, u32)>,
    #[cfg(feature = "presentation")]
    renderer_enabled: bool,
    #[cfg(feature = "presentation")]
    ui_enabled: bool,
    #[cfg(feature = "web")]
    web_enabled: bool,
    modules: Vec<Box<dyn Module>>,
    saves_dir: PathBuf,
    persistence_read_only: bool,
    extension_budget: BudgetLimits,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            extensions_dir: None,
            catalog_extensions: Vec::new(),
            startup_extensions: Vec::new(),
            extension_controller: None,
            logger: Logger::default(),
            tick_hz: DEFAULT_TICK_HZ,
            #[cfg(feature = "presentation")]
            window: None,
            #[cfg(feature = "presentation")]
            renderer_enabled: false,
            #[cfg(feature = "presentation")]
            ui_enabled: false,
            #[cfg(feature = "web")]
            web_enabled: false,
            modules: Vec::new(),
            saves_dir: PathBuf::from("saves"),
            persistence_read_only: false,
            extension_budget: BudgetLimits::default(),
        }
    }

    /// Where `.wasm` extensions are discovered (`build`'s own doc comment).
    /// A relative path resolves against the running executable's own
    /// directory, not the process's cwd (`resolve_relative_to_exe`) -- the
    /// same convention `saves_dir` uses. No default: unset means no
    /// extensions load.
    pub fn extensions_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.extensions_dir = Some(path.into());
        self
    }

    /// Adds one explicitly named component to the extension catalog.
    ///
    /// This complements directory discovery for embedders whose validated
    /// extension catalog spans multiple roots.
    pub fn catalog_extension(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.catalog_extensions.push((name.into(), path.into()));
        self
    }

    /// Restricts initial activation to the named catalog entries.
    ///
    /// With no names configured, every discovered extension loads as before.
    pub fn startup_extension(mut self, name: impl Into<String>) -> Self {
        self.startup_extensions.push(name.into());
        self
    }

    /// Authorizes one host-stamped extension sender to issue runtime
    /// load/unload/reload commands. Unset means runtime control is disabled.
    pub fn extension_controller(mut self, name: impl Into<String>) -> Self {
        self.extension_controller = Some(name.into());
        self
    }

    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    /// Target rate for `run`'s loop. Extensions that never subscribe to
    /// `core/tick` (ADR-004) are unaffected by this either way.
    pub fn tick_hz(mut self, hz: f64) -> Self {
        self.tick_hz = hz;
        self
    }

    /// Opens one SDL window (platform/design.md) and feeds its keyboard
    /// events onto `input/*` each frame of `run`'s loop. No window
    /// configured here means no platform at all — a headless engine.
    #[cfg(feature = "presentation")]
    pub fn window(mut self, title: impl Into<String>, width: u32, height: u32) -> Self {
        self.window = Some((title.into(), width, height));
        self
    }

    /// Attaches a renderer to the window (design/modules.md, ADR-002):
    /// executes `gfx/*` draw commands published by extensions and presents
    /// once per `run` iteration. Requires `.window(...)` — `build`/`run`
    /// error if this is set without one.
    #[cfg(feature = "presentation")]
    pub fn renderer(mut self) -> Self {
        self.renderer_enabled = true;
        self
    }

    /// Attaches the egui ui module (ADR-005, design/presentation.md):
    /// decodes `ui/spec` messages and draws them each `run` iteration.
    /// Requires `.renderer()` too (ui draws through it, direct-wired for
    /// now — see docs/structure.md) — `build`/`run` error if this is set
    /// without one.
    #[cfg(feature = "presentation")]
    pub fn ui(mut self) -> Self {
        self.ui_enabled = true;
        self
    }

    /// Attaches the optional wry web-panel module (ADR-006). Requires
    /// `.window(...)`; it may share that parent with `.renderer()`.
    #[cfg(feature = "web")]
    pub fn web(mut self) -> Self {
        self.web_enabled = true;
        self
    }

    /// Registers a custom native module (design/modules.md, ADR-017):
    /// runs `init` in registration order at `build()` time, then hooks its
    /// `render`/`present` each `run` iteration. The app is built solely on
    /// this same method (via `.renderer()`/`.ui()`'s sugar) — no access an
    /// embedder lacks.
    // TODO: a module registered this way has no way to receive `Engine`'s
    // configured `Logger` at construction time (only `.renderer()`/`.ui()`'s
    // hardcoded sugar does) — `core/audio` accepts this as "no logger for
    // now" rather than inventing a mismatched one of their own; a real fix
    // needs this method's own signature to change (e.g. a factory closure
    // receiving the logger).
    pub fn module(mut self, module: impl Module + 'static) -> Self {
        self.modules.push(Box::new(module));
        self
    }

    /// Where `persistence` (unconditional, see its own doc comment) keeps
    /// `<sender>.bin` save files. Defaults to `"saves"`, relative to the
    /// running executable's own directory if not absolute
    /// (`resolve_relative_to_exe`) -- the same convention `extensions_dir`
    /// uses, not the process's cwd (which a double-clicked or
    /// shortcut-launched binary can't control).
    pub fn saves_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.saves_dir = path.into();
        self
    }

    /// Extensions can still load previously-saved state, but any new
    /// `persistence/save` is silently dropped — a policy choice (an
    /// audited or locked-down extension sandbox), not a resource one; see
    /// `persistence`'s own doc comment for why disabling it outright
    /// wouldn't save anything.
    pub fn read_only_persistence(mut self) -> Self {
        self.persistence_read_only = true;
        self
    }

    /// Sets the per-frame allowances shared by every WASM extension.
    pub fn extension_budget(mut self, limits: BudgetLimits) -> Self {
        self.extension_budget = limits;
        self
    }

    /// Wires the bus, every `.wasm` file in `extensions_dir`, the window
    /// (if `.window(...)` was set), and the renderer (if `.renderer()` was
    /// set). A file that fails to load, or whose name is already taken, is
    /// logged and skipped rather than failing the whole engine. Exposed
    /// publicly (not just used by `run`) for a future driver that wants
    /// the wired-up pieces without `run`'s sleep-loop attached.
    pub fn build(mut self) -> wasmtime::Result<BuiltEngine> {
        #[cfg(feature = "presentation")]
        let window = self.window.take();
        #[cfg(feature = "presentation")]
        let renderer_enabled = self.renderer_enabled;
        #[cfg(feature = "presentation")]
        let ui_enabled = self.ui_enabled;
        let bus = bus::Bus::new();
        let registry = Registry::new();
        let wasm_engine = wasm_extensions::host::new_engine()?;
        let exit_requested = Arc::new(AtomicBool::new(false));

        #[cfg(feature = "presentation")]
        let mut platform = match window {
            Some((title, width, height)) => {
                Some(platform::Platform::new(&title, width, height).map_err(wasmtime::Error::msg)?)
            }
            None => None,
        };
        // Queried once here, independent of the window hand-off below
        // (`Platform` resolves it at construction, not from the live
        // window) - every loaded extension gets the same static snapshot.
        #[cfg(feature = "presentation")]
        let display_info = match &platform {
            Some(platform) => DisplayInfo {
                modes: platform.display_modes().to_vec(),
                native: platform.native_display_mode(),
            },
            None => DisplayInfo::default(),
        };
        #[cfg(not(feature = "presentation"))]
        let display_info = DisplayInfo::default();

        // Build-time-only (ADR-017): every module's `init` runs against
        // this. Seeded with `window-surface` unconditionally (not just for
        // `.renderer()`) so a `.module(...)`-injected replacement renderer
        // can consume it too, same as the built-in one — no privileged
        // access (design/modules.md). Reclaimed by `platform` below if
        // nothing ends up consuming it, so an unclaimed window stays open
        // instead of closing with the registry it briefly lived in.
        let mut services = ServiceRegistry::new();
        #[cfg(feature = "presentation")]
        if let Some(platform) = &mut platform {
            platform.provide_window(&mut services);
        }
        // `bus` service: lets a `.module(...)`-injected module (game-core
        // is the first) publish, not just receive — the same no-privileged-
        // access stance as `window-surface`. `Bus` is cheap to clone (an
        // `Arc` internally), so providing it doesn't compete with anything
        // else that also wants a `Bus` handle.
        services
            .provide(bus.clone())
            .expect("no other service registers as Bus");

        #[cfg(feature = "web")]
        let web_module: Option<Box<dyn Module>> = if self.web_enabled {
            if platform.is_none() {
                return Err(wasmtime::Error::msg(".web() needs .window(...) too"));
            }
            let window = services
                .get()
                .ok_or_else(|| wasmtime::Error::msg("web needs the window-surface service"))?;
            let backend = web::WryBackend::new(window).map_err(wasmtime::Error::msg)?;
            Some(Box::new(web::Web::new(
                bus.clone(),
                self.logger.clone(),
                backend,
            )))
        } else {
            None
        };

        #[cfg(feature = "presentation")]
        let renderer = if renderer_enabled {
            if platform.is_none() {
                return Err(wasmtime::Error::msg(".renderer() needs .window(...) too"));
            }
            let shared = Arc::new(Mutex::new(Renderer::new(bus.clone(), self.logger.clone())));
            {
                let mut renderer = shared.lock().unwrap();
                let mut ctx = ModuleContext::new(&mut services);
                Module::init(&mut *renderer, &mut ctx).map_err(wasmtime::Error::msg)?;
            }
            let ep = bus.register("renderer", Shared(shared.clone()));
            ep.subscribe(GFX_TOPICS);
            ep.subscribe(bones_messages::lifecycle::LifecycleEvent::TOPIC);
            Some(shared)
        } else {
            None
        };

        #[cfg(feature = "presentation")]
        let ui = if ui_enabled {
            renderer
                .as_ref()
                .ok_or_else(|| wasmtime::Error::msg(".ui() needs .renderer() too"))?;
            let shared = Arc::new(Mutex::new(Ui::new(bus.clone(), self.logger.clone())));
            let ep = bus.register("ui", Shared(shared.clone()));
            ep.subscribe(bones_messages::ui::Spec::TOPIC);
            Some(shared)
        } else {
            None
        };

        // Native modules register (bus + call registry) before any
        // extension loads below, deliberately — an extension's own `init`
        // can `send` a module synchronously (ADR-010, e.g. persistence's
        // load-on-init), and that only reaches anything if the target is
        // already registered by the time it's called. Discovered as a real
        // bug (not a hypothetical) building the persistence_demo
        // extension: it always loaded as "nothing saved" because
        // `persistence` registered after every extension's `init` had
        // already run and failed its `send` with `SendError::UnknownEndpoint`.
        let mut modules = Vec::new();
        #[cfg(feature = "web")]
        if let Some(module) = web_module {
            register_module(&bus, &registry, &mut services, &mut modules, module)
                .map_err(wasmtime::Error::msg)?;
        }
        for module in self.modules.drain(..) {
            register_module(&bus, &registry, &mut services, &mut modules, module)
                .map_err(wasmtime::Error::msg)?;
        }

        // Unconditional (persistence's own doc comment explains why) —
        // registered here, not through `self.modules`, so there's no
        // `.persistence()`-style opt-in to forget.
        let saves_dir = resolve_relative_to_exe(self.saves_dir.clone());
        let persistence = Persistence::new(saves_dir, self.persistence_read_only);
        register_module(
            &bus,
            &registry,
            &mut services,
            &mut modules,
            Box::new(persistence),
        )
        .map_err(wasmtime::Error::msg)?;

        let mut tracked = Vec::new();
        let mut catalog = std::collections::HashMap::new();
        let mut catalog_entries = Vec::new();

        if let Some(dir) = &self.extensions_dir {
            let dir = resolve_relative_to_exe(dir.clone());
            for path in find_wasm_files(&dir) {
                catalog_entries.push((derive_extension_name(&path), path));
            }
        }
        catalog_entries.extend(
            self.catalog_extensions
                .drain(..)
                .map(|(name, path)| (name, resolve_relative_to_exe(path))),
        );
        for (name, path) in catalog_entries {
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
            if !self.startup_extensions.is_empty() && !self.startup_extensions.contains(&name) {
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
                self.extension_budget,
            ) {
                Ok((ep, shared, budget, topics)) => {
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
                        endpoint: ep,
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
                }
            }
        }
        for name in &self.startup_extensions {
            if !catalog.contains_key(name) {
                self.logger.error(
                    "engine",
                    &format!("startup extension '{name}' is not in the catalog"),
                );
            }
        }

        let commands = Arc::new(Mutex::new(Vec::new()));
        let command_sink = commands.clone();
        let controller = self.extension_controller.clone();
        let control_logger = self.logger.clone();
        let control = bus.register("extension-manager", move |envelope: &bus::Envelope| {
            if controller.as_deref() != Some(envelope.sender.as_str()) {
                control_logger.error(
                    "engine",
                    &format!(
                        "rejected runtime extension command from '{}' on '{}'",
                        envelope.sender, envelope.topic
                    ),
                );
                return;
            }
            match bones_messages::extension_control::Command::decode(
                &envelope.topic,
                &envelope.payload,
            ) {
                Ok(Some(command)) => command_sink
                    .lock()
                    .unwrap()
                    .push(crate::supervisor::OwnedCommand::from(command)),
                Ok(None) => control_logger.warn(
                    "engine",
                    &format!(
                        "ignored unknown extension command topic '{}'",
                        envelope.topic
                    ),
                ),
                Err(err) => control_logger.warn(
                    "engine",
                    &format!(
                        "could not decode extension command from '{}' on '{}': {err}",
                        envelope.sender, envelope.topic
                    ),
                ),
            }
        });
        control.subscribe("core/extensions/*");

        let supervisor = Supervisor::new(
            wasm_engine,
            bus.clone(),
            registry,
            self.logger.clone(),
            tracked,
            catalog,
            commands,
            exit_requested.clone(),
            display_info,
            self.extension_budget,
        );

        #[cfg(feature = "presentation")]
        if let Some(platform) = &mut platform {
            platform.reclaim_window(&mut services);
        }

        Ok(BuiltEngine {
            runner: Runner::new(bus, self.logger),
            #[cfg(feature = "presentation")]
            platform,
            #[cfg(feature = "presentation")]
            renderer,
            #[cfg(feature = "presentation")]
            ui,
            modules,
            supervisor,
            exit_requested,
            shutdown_started: false,
        })
    }

    /// Runs at `tick_hz`, real wall-clock timing, until the window is
    /// closed (or forever, if `.window(...)` was never set) — a thin
    /// wrapper around `Runner::step` (ADR-014). `Supervisor::check` runs
    /// both before and after `step`: before, so a swapped-in extension's
    /// first tick already runs against the new code; after, so a fault
    /// from that same tick is quarantined this iteration rather than the
    /// next one.
    pub fn run(self) -> wasmtime::Result<()> {
        let period = Duration::from_secs_f64(1.0 / self.tick_hz);
        let BuiltEngine {
            runner,
            #[cfg(feature = "presentation")]
            mut platform,
            #[cfg(feature = "presentation")]
            renderer,
            #[cfg(feature = "presentation")]
            ui,
            modules,
            mut supervisor,
            exit_requested,
            shutdown_started: _,
        } = self.build()?;

        let mut last = std::time::Instant::now() - period;
        let shutdown_sender = loop {
            #[cfg(feature = "presentation")]
            if let Some(platform) = &mut platform {
                // ADR-008: offer every raw event to the ui layer first; what
                // it claims (wants_pointer_input/wants_keyboard_input, as of
                // the end of the last `update`) never reaches `input/*`.
                // Locked once for the whole poll, not per event.
                let mut ui_guard = ui.as_ref().map(|ui| ui.lock().unwrap());
                platform.poll_events_with(runner.bus(), "platform", |event| {
                    ui_guard.as_mut().is_some_and(|ui| ui.feed_event(event))
                });
                if platform.quit_requested() {
                    break "platform";
                }
            }
            if exit_requested.load(Ordering::Relaxed) {
                break ENGINE_SENDER;
            }

            supervisor.check();

            let now = std::time::Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            runner.step(dt);
            supervisor.check();

            // render phase (design/modules.md): gfx/ui draws already
            // happened synchronously via `Handler::handle` during `step`'s
            // dispatch above, so `render` is a no-op for renderer today —
            // still called for any module that does need it.
            #[cfg(feature = "presentation")]
            if let Some(renderer) = &renderer {
                renderer.lock().unwrap().render();
            }
            for module in &modules {
                module.lock().unwrap().render();
            }

            // ui draws above every gfx layer (design/presentation.md), so
            // its `update` runs between `render` and `present`.
            #[cfg(feature = "presentation")]
            if let (Some(ui), Some(renderer)) = (&ui, &renderer) {
                let mut renderer = renderer.lock().unwrap();
                let (width, height) = renderer.size();
                ui.lock().unwrap().update(&mut renderer, width, height);
            }

            #[cfg(feature = "presentation")]
            if let Some(renderer) = &renderer {
                renderer.lock().unwrap().present();
            }
            for module in &modules {
                module.lock().unwrap().present();
            }

            let elapsed = now.elapsed();
            if elapsed < period {
                std::thread::sleep(period - elapsed);
            }
        };

        run_shutdown(&runner, &mut supervisor, &modules, shutdown_sender);
        Ok(())
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
