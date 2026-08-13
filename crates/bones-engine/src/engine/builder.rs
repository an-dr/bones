//! The public builder API (design/modules.md, ADR-017): discovers WASM
//! extensions and runs them, plus `.module(...)` for injecting custom
//! native modules.
//!
//! This is the composition root, and the only reason `bones-engine` holds
//! code at all rather than being a pure re-export: everything module-
//! agnostic — the frame loop, extension loading, extension supervision —
//! lives in `bones-kernel`, and what remains here is precisely the part
//! that names concrete `bones-module-*` types.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bones_kernel::bus::{BudgetLimits, Module, Registry, ServiceRegistry};
use bones_kernel::logging::Logger;
use bones_kernel::wasm_extensions::files::{self, Files};
use bones_kernel::wasm_extensions::host::{DisplayInfo, ExtensionTimeouts};
use bones_kernel::wasm_extensions::lifecycle;
use bones_kernel::wasm_extensions::lifecycle::Event;
use bones_kernel::wasm_extensions::persistence::Persistence;
#[cfg(feature = "presentation")]
use bones_module_renderer::Renderer;
#[cfg(feature = "presentation")]
use bones_module_ui::Ui;

use bones_kernel::runner::Runner;
use bones_kernel::wasm_extensions::loading::{
    attach_extension, derive_extension_name, find_wasm_files, read_file_mtime, ENGINE_SENDER,
};
use bones_kernel::wasm_extensions::supervisor::Supervisor;
use bones_kernel::wasm_extensions::supervisor::TrackedExtension;

use super::built_engine::{run_frame_phases, run_shutdown, BuiltEngine};
use super::register_module::register_module;

const DEFAULT_TICK_HZ: f64 = 60.0;

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

/// The frame period `run` sleeps to, or an error naming the offending rate.
///
/// `Duration::from_secs_f64` panics on a negative, NaN, or overflowing value,
/// and `1.0 / tick_hz` produces exactly those from a zero, negative, or NaN
/// rate. A panic is the wrong failure here: `tick_hz` takes an `f64` that
/// routinely comes from a config file, so an invalid one is ordinary bad
/// input, and this builder already has a `Result` to report it through.
///
/// The overflow arm is not theoretical either — a rate small enough to be
/// finite still divides into a period no `Duration` can hold.
fn tick_period(tick_hz: f64) -> crate::Result<Duration> {
    if !tick_hz.is_finite() || tick_hz <= 0.0 {
        return Err(crate::Error::msg(format!(
            "tick_hz must be finite and greater than zero, got {tick_hz}"
        )));
    }
    Duration::try_from_secs_f64(1.0 / tick_hz).map_err(|err| {
        crate::Error::msg(format!(
            "tick_hz {tick_hz} is too small to be a frame period: {err}"
        ))
    })
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// The composition root: everything the engine will be made of, before it is
/// made (design/modules.md, ADR-017).
///
/// - Every method takes and returns `self`, so a composition reads as one
///   chain and nothing is half-configured in between.
/// - Nothing here touches the OS or loads a component. `build` does that and
///   reports what went wrong; the chain itself cannot fail.
/// - Native modules arrive through `.module(...)`, including the ones the
///   engine ships — `.renderer()` and `.ui()` are sugar over it, so an
///   injected module has no less access than a built-in one.
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
    min_window_size: Option<(u32, u32)>,
    #[cfg(feature = "presentation")]
    renderer_enabled: bool,
    #[cfg(feature = "presentation")]
    ui_enabled: bool,
    #[cfg(feature = "web")]
    web_enabled: bool,
    modules: Vec<Box<dyn Module>>,
    saves_dir: PathBuf,
    persistence_read_only: bool,
    files_root: Option<PathBuf>,
    extension_budget: BudgetLimits,
    extension_timeouts: ExtensionTimeouts,
}

impl Engine {
    /// An engine with no window, no extensions directory, no native modules,
    /// and a 60 Hz tick — a headless engine that builds and steps as-is.
    ///
    /// Everything beyond that is opt-in, so what a composition does is what
    /// its own chain says, with nothing inherited from a default that is not
    /// written down at the call site.
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
            min_window_size: None,
            #[cfg(feature = "presentation")]
            renderer_enabled: false,
            #[cfg(feature = "presentation")]
            ui_enabled: false,
            #[cfg(feature = "web")]
            web_enabled: false,
            modules: Vec::new(),
            saves_dir: PathBuf::from("states"),
            persistence_read_only: false,
            files_root: None,
            extension_budget: BudgetLimits::default(),
            extension_timeouts: ExtensionTimeouts::default(),
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

    /// The sink every engine log line goes to (ADR-012), resolved once at
    /// construction rather than looked up per call.
    ///
    /// Defaults to `Logger::default()`, which writes to stdout. Pass a clone
    /// of the same `Logger` to a module that logs, so the whole composition
    /// shares one destination.
    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    /// Target rate for `run`'s loop. Extensions that never subscribe to
    /// `core/tick` (ADR-004) are unaffected by this either way.
    ///
    /// Must be finite and greater than zero. An invalid rate is reported by
    /// `build`/`run` rather than here, so a builder chain stays infallible
    /// and the error arrives with every other composition error.
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

    /// Floors how small `.window(...)`'s window can be resized. No effect
    /// without `.window(...)` — `build` applies it only if a window opened.
    #[cfg(feature = "presentation")]
    pub fn min_window_size(mut self, width: u32, height: u32) -> Self {
        self.min_window_size = Some((width, height));
        self
    }

    /// Attaches a renderer to the window (design/modules.md, ADR-002):
    /// executes `gfx/*` draw commands published by extensions and presents
    /// once per `run` iteration. Requires `.window(...)` — `build`/`run`
    /// error if this is set without one.
    ///
    /// Sugar for `.module(Renderer::new(...))`, registered ahead of the
    /// injected modules so its `draw-target` service exists before
    /// anything can consume it.
    #[cfg(feature = "presentation")]
    pub fn renderer(mut self) -> Self {
        self.renderer_enabled = true;
        self
    }

    /// Attaches the egui ui module (ADR-005, design/presentation.md):
    /// decodes `ui/spec` messages and draws them each `run` iteration.
    ///
    /// Sugar for `.module(Ui::new(...))`. It needs *some* module to have
    /// provided the `draw-target` service, which `.renderer()` does —
    /// `build` errors if nothing did.
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
    /// `filter_event`/`render`/`present` each `run` iteration. Every module
    /// the engine ships arrives through this same method — including
    /// `.renderer()`'s and `.ui()`'s, which are sugar over it — so an
    /// injected module has no less access than a built-in one, and either
    /// can be replaced by the other.
    // TODO: a module registered this way has no way to receive `Engine`'s
    // configured `Logger` at construction time — only the `.renderer()`/
    // `.ui()` sugar can, because it constructs the module itself. `audio`
    // accepts this as "no logger for now" rather than inventing a
    // mismatched one of its own; a real fix needs this method's own
    // signature to change (e.g. a factory closure receiving the logger).
    pub fn module(mut self, module: impl Module + 'static) -> Self {
        self.modules.push(Box::new(module));
        self
    }

    /// Where `persistence` (unconditional, see its own doc comment) keeps
    /// `<sender>.bin` save files. Defaults to `"states"`, relative to the
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

    /// Grants extensions read access to one directory through the `files`
    /// endpoint (`bones_kernel::wasm_extensions::files`). A relative path resolves against
    /// the running executable's own directory, the same convention
    /// `saves_dir` uses.
    ///
    /// No default: unset means the capability does not exist, which is the
    /// right default for a grant over someone else's files. Reads are capped
    /// at `files::DEFAULT_MAX_BYTES` per file.
    pub fn files_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.files_root = Some(path.into());
        self
    }

    /// Sets the per-frame allowances shared by every WASM extension.
    pub fn extension_budget(mut self, limits: BudgetLimits) -> Self {
        self.extension_budget = limits;
        self
    }

    /// Sets how long `instantiate` + `init` may take per extension, for the
    /// initial load and for every hot reload after it.
    ///
    /// - Defaults to `ExtensionTimeouts::default().load`, which suits a small
    ///   component.
    /// - Raise it for a large one — megabytes carrying an embedded language
    ///   runtime routinely need longer than a second to start.
    /// - The budget is wall clock, so it must also absorb the host being
    ///   busy: an extension that loads on an idle machine can still miss a
    ///   tight deadline on a loaded one, and a missed deadline is a trap in
    ///   `init`, not a retry.
    pub fn extension_load_timeout(mut self, timeout: Duration) -> Self {
        self.extension_timeouts.load = timeout;
        self
    }

    /// Sets how long any single guest call may take -- `on-message`,
    /// `on-tick`, or answering a direct `send`.
    ///
    /// - Defaults to `ExtensionTimeouts::default().call`, tight enough that a
    ///   call blocking that long reads as a runaway.
    /// - Raise it where that assumption is wrong: an extension whose messages
    ///   trigger real work (reading a repository, parsing a large document)
    ///   is judged on its slowest call, not its typical one.
    /// - Overrunning faults the extension and quarantines it, so the cost of
    ///   setting this too low is an app that stops responding mid-session,
    ///   not one that merely stutters.
    pub fn extension_call_timeout(mut self, timeout: Duration) -> Self {
        self.extension_timeouts.call = timeout;
        self
    }

    /// Wires the bus, every `.wasm` file in `extensions_dir`, the window
    /// (if `.window(...)` was set), and the renderer (if `.renderer()` was
    /// set). A file that fails to load, or whose name is already taken, is
    /// logged and skipped rather than failing the whole engine. Exposed
    /// publicly (not just used by `run`) for a future driver that wants
    /// the wired-up pieces without `run`'s sleep-loop attached.
    pub fn build(mut self) -> crate::Result<BuiltEngine> {
        // Validated here, not only in `run`: a caller driving `Runner::step`
        // itself never calls `run`, and reads `tick_hz` back as the rate it
        // is meant to step at. Rejecting it at the same point as every other
        // composition error keeps that caller from inheriting a rate the
        // engine already knows is unusable.
        tick_period(self.tick_hz)?;

        #[cfg(feature = "presentation")]
        let window = self.window.take();
        #[cfg(feature = "presentation")]
        let renderer_enabled = self.renderer_enabled;
        #[cfg(feature = "presentation")]
        let ui_enabled = self.ui_enabled;
        let bus = bones_kernel::bus::Bus::new();
        let registry = Registry::new();
        let wasm_engine = bones_kernel::wasm_extensions::host::new_engine()?;
        let exit_requested = Arc::new(AtomicBool::new(false));

        #[cfg(feature = "presentation")]
        let min_window_size = self.min_window_size.take();
        #[cfg(feature = "presentation")]
        let mut platform = match window {
            Some((title, width, height)) => {
                let mut platform = bones_kernel::platform::Platform::new(&title, width, height)
                    .map_err(crate::Error::msg)?;
                if let Some((min_width, min_height)) = min_window_size {
                    platform
                        .set_min_size(min_width, min_height)
                        .map_err(crate::Error::msg)?;
                }
                Some(platform)
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
        // `registry` service: the direct-send half of the same stance.
        // `register_module` already inserts every module as a *callable*
        // target, so modules could be called by name but had no way to call
        // anything themselves — an asymmetry against WASM extensions, which
        // get `send` as a host import. Modules registered before extensions
        // attach, so this is also the only way to reach an endpoint like
        // `web` during the window between the window opening and the first
        // extension finishing `init`.
        services
            .provide(registry.clone())
            .expect("no other service registers as Registry");

        #[cfg(feature = "web")]
        let web_module: Option<Box<dyn Module>> = if self.web_enabled {
            if platform.is_none() {
                return Err(crate::Error::msg(".web() needs .window(...) too"));
            }
            let window = services
                .get()
                .ok_or_else(|| crate::Error::msg("web needs the window-surface service"))?;
            let backend = bones_module_web::WryBackend::new(window).map_err(crate::Error::msg)?;
            Some(Box::new(bones_module_web::Web::new(
                bus.clone(),
                self.logger.clone(),
                backend,
            )))
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

        // `.renderer()` and `.ui()` are sugar over `.module(...)`, not a
        // second way in: both go through `register_module` like anything
        // an embedder injects, and neither is named again after this
        // point. Registration order is the only thing that makes them the
        // first two — the renderer provides `draw-target`, which ui's own
        // `init` then consumes, and the render/present phases run modules
        // in this same order so ui draws above the gfx layers.
        #[cfg(feature = "presentation")]
        if renderer_enabled {
            // Checked here as well as in the module's own `init`, which
            // reports the missing `window-surface` service. That message is
            // right for a `.module(...)` caller and wrong for this one: an
            // embedder who typed `.renderer()` has never heard of the
            // service, and needs to be told about `.window(...)`.
            if platform.is_none() {
                return Err(crate::Error::msg(".renderer() needs .window(...) too"));
            }
            let renderer = Renderer::new(bus.clone(), self.logger.clone());
            register_module(
                &bus,
                &registry,
                &mut services,
                &mut modules,
                Box::new(renderer),
            )
            .map_err(crate::Error::msg)?;
        }
        #[cfg(feature = "presentation")]
        if ui_enabled {
            let ui = Ui::new(bus.clone(), self.logger.clone());
            register_module(&bus, &registry, &mut services, &mut modules, Box::new(ui))
                .map_err(crate::Error::msg)?;
        }

        #[cfg(feature = "web")]
        if let Some(module) = web_module {
            register_module(&bus, &registry, &mut services, &mut modules, module)
                .map_err(crate::Error::msg)?;
        }
        for module in self.modules.drain(..) {
            register_module(&bus, &registry, &mut services, &mut modules, module)
                .map_err(crate::Error::msg)?;
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
        .map_err(crate::Error::msg)?;

        // Opt-in, unlike persistence: without a granted root there is no
        // endpoint, so an extension's read fails as an unknown endpoint.
        if let Some(root) = self.files_root.clone() {
            let files = Files::new(resolve_relative_to_exe(root), files::DEFAULT_MAX_BYTES);
            register_module(
                &bus,
                &registry,
                &mut services,
                &mut modules,
                Box::new(files),
            )
            .map_err(crate::Error::msg)?;
        }

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
                self.extension_timeouts,
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
        let control = bus.register(
            "extension-manager",
            move |envelope: &bones_kernel::bus::Envelope| {
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
                    Ok(Some(command)) => command_sink.lock().unwrap().push(
                        bones_kernel::wasm_extensions::supervisor::OwnedCommand::from(command),
                    ),
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
            },
        );
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
            self.extension_timeouts,
        );

        #[cfg(feature = "presentation")]
        if let Some(platform) = &mut platform {
            platform.reclaim_window(&mut services);
        }

        Ok(BuiltEngine {
            runner: Runner::new(bus, self.logger),
            #[cfg(feature = "presentation")]
            platform,
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
    pub fn run(self) -> crate::Result<()> {
        let period = tick_period(self.tick_hz)?;
        let BuiltEngine {
            runner,
            #[cfg(feature = "presentation")]
            mut platform,
            modules,
            mut supervisor,
            exit_requested,
            shutdown_started: _,
        } = self.build()?;

        let mut last = std::time::Instant::now() - period;
        let shutdown_sender = loop {
            #[cfg(feature = "presentation")]
            if let Some(platform) = &mut platform {
                // ADR-008: offer every raw event to the modules first, in
                // registration order, and stop at the first one that claims
                // it — what a module claims never reaches `input/*`. No
                // module is named here: a `.module(...)`-injected overlay
                // filters input on exactly the same terms the built-in ui
                // does.
                platform.poll_events_with(runner.bus(), "platform", |event| {
                    bones_kernel::bus::offer_event(&modules, event)
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

            // render then present, both over every module in registration
            // order (design/modules.md). The renderer composites its
            // retained `gfx/*` batches, ui draws above that, and only then
            // does the frame flip — layering that comes from the order
            // modules were composed in, not from naming any of them here.
            run_frame_phases(&modules);

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
