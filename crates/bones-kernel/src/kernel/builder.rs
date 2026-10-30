//! The public builder API (design/modules.md, ADR-017): discovers WASM
//! extensions and runs them, plus `.module(...)` for injecting custom
//! native modules.
//!
//! This is the composition root, and the only reason `bones-kernel` holds
//! code at all rather than being a pure re-export: everything module-
//! agnostic — the frame loop, extension loading, extension supervision —
//! lives in `bones-kernel-infra`, and what remains here is precisely the part
//! that names concrete `bones-module-*` types.

use std::path::{Path, PathBuf};

use std::time::Duration;

/// The predicate `devtools_redacting` stores until `build` hands it on.
#[cfg(feature = "devtools")]
type DevtoolsRedact = Box<dyn Fn(&bones_kernel_infra::bus::Envelope) -> bool + Send + Sync>;

#[cfg(feature = "wasm")]
use bones_kernel_infra::bus::BudgetLimits;
use bones_kernel_infra::bus::{Module, Registry, ServiceRegistry};
use bones_kernel_infra::display::DisplayInfo;
use bones_kernel_infra::files::{self, Files};
use bones_kernel_infra::logging::Logger;
use bones_kernel_infra::persistence::Persistence;
#[cfg(feature = "presentation")]
use bones_module_renderer::Renderer;
#[cfg(feature = "presentation")]
use bones_module_ui::Ui;
#[cfg(feature = "wasm")]
use bones_module_wasm::host::ExtensionTimeouts;

use bones_kernel_infra::runner::Runner;

use super::built_kernel::{run_frame_phases, run_shutdown, step_frame, BuiltKernel, ENGINE_SENDER};
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
pub struct Kernel {
    #[cfg(feature = "wasm")]
    extensions_dir: Option<PathBuf>,
    #[cfg(feature = "wasm")]
    catalog_extensions: Vec<(String, PathBuf)>,
    #[cfg(feature = "wasm")]
    startup_extensions: Vec<String>,
    #[cfg(feature = "wasm")]
    required_extensions: Vec<String>,
    #[cfg(feature = "wasm")]
    extension_controller: Option<String>,
    logger: Logger,
    tick_hz: f64,
    #[cfg(feature = "presentation")]
    window: Option<(String, u32, u32)>,
    #[cfg(feature = "presentation")]
    min_window_size: Option<(u32, u32)>,
    #[cfg(feature = "presentation")]
    offscreen: bool,
    #[cfg(feature = "presentation")]
    offscreen_variable: Option<String>,
    #[cfg(feature = "presentation")]
    renderer_enabled: bool,
    #[cfg(feature = "presentation")]
    ui_enabled: bool,
    #[cfg(feature = "web")]
    web_enabled: bool,
    #[cfg(feature = "instance")]
    instance: Option<bones_module_single_instance::Primary>,
    #[cfg(feature = "devtools")]
    devtools_variable: Option<String>,
    #[cfg(feature = "devtools")]
    devtools_redact: Option<DevtoolsRedact>,
    modules: Vec<Box<dyn Module>>,
    saves_dir: PathBuf,
    persistence_read_only: bool,
    files_root: Option<PathBuf>,
    #[cfg(feature = "wasm")]
    extension_budget: BudgetLimits,
    #[cfg(feature = "wasm")]
    extension_timeouts: ExtensionTimeouts,
    #[cfg(feature = "wasm")]
    extension_permissions: Vec<(String, bones_module_wasm::Permissions)>,
}

impl Kernel {
    /// An engine with no window, no extensions directory, no native modules,
    /// and a 60 Hz tick — a headless engine that builds and steps as-is.
    ///
    /// Everything beyond that is opt-in, so what a composition does is what
    /// its own chain says, with nothing inherited from a default that is not
    /// written down at the call site.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "wasm")]
            extensions_dir: None,
            #[cfg(feature = "wasm")]
            catalog_extensions: Vec::new(),
            #[cfg(feature = "wasm")]
            startup_extensions: Vec::new(),
            #[cfg(feature = "wasm")]
            required_extensions: Vec::new(),
            #[cfg(feature = "wasm")]
            extension_controller: None,
            logger: Logger::default(),
            tick_hz: DEFAULT_TICK_HZ,
            #[cfg(feature = "presentation")]
            window: None,
            #[cfg(feature = "presentation")]
            min_window_size: None,
            #[cfg(feature = "presentation")]
            offscreen: false,
            #[cfg(feature = "presentation")]
            offscreen_variable: None,
            #[cfg(feature = "presentation")]
            renderer_enabled: false,
            #[cfg(feature = "presentation")]
            ui_enabled: false,
            #[cfg(feature = "web")]
            web_enabled: false,
            #[cfg(feature = "instance")]
            instance: None,
            #[cfg(feature = "devtools")]
            devtools_variable: None,
            #[cfg(feature = "devtools")]
            devtools_redact: None,
            modules: Vec::new(),
            saves_dir: PathBuf::from("states"),
            persistence_read_only: false,
            files_root: None,
            #[cfg(feature = "wasm")]
            extension_budget: BudgetLimits::default(),
            #[cfg(feature = "wasm")]
            extension_timeouts: ExtensionTimeouts::default(),
            #[cfg(feature = "wasm")]
            extension_permissions: Vec::new(),
        }
    }

    #[cfg(feature = "wasm")]
    /// Where `.wasm` extensions are discovered (`build`'s own doc comment).
    /// A relative path resolves against the running executable's own
    /// directory, not the process's cwd (`resolve_relative_to_exe`) -- the
    /// same convention `saves_dir` uses. No default: unset means no
    /// extensions load.
    pub fn extensions_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.extensions_dir = Some(path.into());
        self
    }

    #[cfg(feature = "wasm")]
    /// Adds one explicitly named component to the extension catalog.
    ///
    /// This complements directory discovery for embedders whose validated
    /// extension catalog spans multiple roots.
    pub fn catalog_extension(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.catalog_extensions.push((name.into(), path.into()));
        self
    }

    #[cfg(feature = "wasm")]
    /// Restricts initial activation to the named catalog entries.
    ///
    /// With no names configured, every discovered extension loads as before.
    pub fn startup_extension(mut self, name: impl Into<String>) -> Self {
        self.startup_extensions.push(name.into());
        self
    }

    #[cfg(feature = "wasm")]
    /// Loads the named catalog entry at startup, as [`startup_extension`](Self::startup_extension)
    /// does, and makes `build` fail when it is not in the catalog or does
    /// not load. Otherwise such a failure is only logged and the engine runs
    /// on without it -- for an app whose window that extension drives, a
    /// blank window with no error.
    pub fn required_extension(mut self, name: impl Into<String>) -> Self {
        self.required_extensions.push(name.into());
        self
    }

    #[cfg(feature = "wasm")]
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

    /// Opens `.window(...)`'s window in memory instead of on a display,
    /// through SDL's offscreen video driver, so an engine with a renderer
    /// runs where there is no display: a test, a CI job, or a screenshot
    /// taken for documentation. `renderer/capture` reads its frames back.
    ///
    /// The driver is chosen by an SDL hint that lasts for the whole process
    /// (`Platform::select_offscreen_video_driver`). No effect without
    /// `.window(...)`. `build` refuses it with `.web()`: a webview needs a
    /// visible parent window.
    #[cfg(feature = "presentation")]
    pub fn offscreen(mut self) -> Self {
        self.offscreen = true;
        self
    }

    /// As [`Kernel::offscreen`] when the environment variable `variable` is
    /// set at `build` to anything but empty or `0`, so a tool can take
    /// screenshots of an application that otherwise opens a visible window.
    #[cfg(feature = "presentation")]
    pub fn offscreen_from_env(mut self, variable: &str) -> Self {
        self.offscreen_variable = Some(variable.to_string());
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

    /// Attaches the optional `os` module: clipboard, browser, native file
    /// dialogs and HTTPS fetch, for guests that have none of their own.
    ///
    /// Needs no window, unlike `.web()` and `.renderer()` -- the dialogs it
    /// opens are the platform's own.
    #[cfg(feature = "os")]
    pub fn os(self) -> Self {
        self.module(bones_module_os::OsModule::default())
    }

    /// Makes this the single instance a later launch hands its arguments to
    /// (ADR-035): each arrives as `instance/launched`, and the window comes
    /// to the front.
    ///
    /// `primary` comes from `instance::claim`, which runs in `main` before
    /// this builder exists -- a launch that handed off has already exited.
    /// Works headless too; there is then no window to raise.
    #[cfg(feature = "instance")]
    pub fn single_instance(mut self, primary: bones_module_single_instance::Primary) -> Self {
        self.instance = Some(primary);
        self
    }

    /// Lets a page or guest register, unregister, or check this application's
    /// desktop registration over `desktop/*` (ADR-036). What is registered --
    /// names, icon, executable -- is `integration`, fixed here.
    #[cfg(feature = "desktop")]
    pub fn desktop(self, integration: bones_module_desktop::Integration<'static>) -> Self {
        self.module(bones_module_desktop::DesktopModule::new(integration))
    }

    /// Lets a page or guest check for, stage, and install updates over
    /// `upgrader/*` (ADR-037). Where releases come from is `config`, fixed
    /// here; a guest chooses only the action.
    #[cfg(feature = "upgrade")]
    pub fn upgrade(self, config: bones_module_upgrade::UpgradeConfig) -> Self {
        self.module(bones_module_upgrade::UpgradeModule::new(config))
    }

    /// Opens the loopback devtools channel when the environment variable
    /// `variable` holds a port number at `build`, and registers nothing at
    /// all otherwise.
    ///
    /// The channel is unauthenticated: name a variable no shipped launch
    /// sets, and read the `devtools` module's README before using it.
    #[cfg(feature = "devtools")]
    pub fn devtools_from_env(mut self, variable: &str) -> Self {
        self.devtools_variable = Some(variable.to_string());
        self
    }

    /// Records only the topic, sender, and length of every envelope
    /// `redact` matches on the channel `devtools_from_env` opens -- for
    /// payloads that can carry credentials. Without `devtools_from_env`, or
    /// with its variable unset, it does nothing.
    #[cfg(feature = "devtools")]
    pub fn devtools_redacting(
        mut self,
        redact: impl Fn(&bones_kernel_infra::bus::Envelope) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.devtools_redact = Some(Box::new(redact));
        self
    }

    /// Registers a custom native module (design/modules.md, ADR-017):
    /// runs `init` in registration order at `build()` time, then hooks its
    /// `filter_event`/`render`/`present` each `run` iteration. Every module
    /// the engine ships arrives through this same method — including
    /// `.renderer()`'s and `.ui()`'s, which are sugar over it — so an
    /// injected module has no less access than a built-in one, and either
    /// can be replaced by the other.
    // TODO: a module registered this way has no way to receive `Kernel`'s
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
    /// endpoint (`bones_kernel_infra::files`). A relative path resolves against
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

    #[cfg(feature = "wasm")]
    /// Sets the per-frame allowances shared by every WASM extension.
    pub fn extension_budget(mut self, limits: BudgetLimits) -> Self {
        self.extension_budget = limits;
        self
    }

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
    /// Denies the extension called `name` the topics and endpoints in
    /// `permissions` (ADR-042).
    ///
    /// - For an extension the application did not write: a plugin, or code
    ///   nobody has reviewed yet.
    /// - Applies to every instance loaded under that name, including hot
    ///   reloads and runtime loads.
    pub fn extension_permissions(
        mut self,
        name: impl Into<String>,
        permissions: crate::wasm::Permissions,
    ) -> Self {
        self.extension_permissions.push((name.into(), permissions));
        self
    }

    /// Wires the bus, every `.wasm` file in `extensions_dir`, the window
    /// (if `.window(...)` was set), and the renderer (if `.renderer()` was
    /// set). A file that fails to load, or whose name is already taken, is
    /// logged and skipped rather than failing the whole engine. Exposed
    /// publicly (not just used by `run`) for a future driver that wants
    /// the wired-up pieces without `run`'s sleep-loop attached.
    pub fn build(mut self) -> crate::Result<BuiltKernel> {
        // Validated here, not only in `run`: a caller driving `Runner::step`
        // itself never calls `run`, and reads `tick_hz` back as the rate it
        // is meant to step at. Rejecting it at the same point as every other
        // composition error keeps that caller from inheriting a rate the
        // engine already knows is unusable.
        tick_period(self.tick_hz)?;

        // Teed before any module takes a copy of the logger, so the channel's
        // `/logs` sees every line the console does.
        #[cfg(feature = "devtools")]
        let devtools_logs = match &self.devtools_variable {
            Some(variable) if bones_module_devtools::Devtools::requested(variable) => {
                let history = bones_module_devtools::LogHistory::new();
                self.logger = self.logger.tee(std::sync::Arc::new(history.clone()));
                Some(history)
            }
            _ => None,
        };

        #[cfg(feature = "presentation")]
        let window = self.window.take();
        #[cfg(feature = "presentation")]
        let renderer_enabled = self.renderer_enabled;
        #[cfg(feature = "presentation")]
        let ui_enabled = self.ui_enabled;
        let bus = bones_kernel_infra::bus::Bus::new();
        let registry = Registry::new();
        let exit_requested = bones_kernel_infra::exit::ExitRequest::new();
        let frame_control = bones_kernel_infra::frame_control::FrameControl::new();

        #[cfg(feature = "presentation")]
        let min_window_size = self.min_window_size.take();
        // Web panels embed X11 child views on Linux; selected before the
        // window exists, and without touching the process environment.
        #[cfg(feature = "web")]
        if self.web_enabled {
            bones_kernel_infra::platform::Platform::select_x11_video_driver();
        }
        #[cfg(feature = "presentation")]
        let offscreen = self.offscreen || env_flag(self.offscreen_variable.as_deref());
        // A webview needs a real parent window; under the offscreen driver
        // there is none, and creating one fails inside the platform.
        #[cfg(feature = "web")]
        if offscreen && self.web_enabled {
            return Err(crate::Error::msg(
                "web panels need a visible window, so this engine cannot run offscreen; \
                 capture the visible window instead (bones-cli screenshot --window)",
            ));
        }
        #[cfg(feature = "presentation")]
        if offscreen {
            bones_kernel_infra::platform::Platform::select_offscreen_video_driver();
        }
        #[cfg(feature = "presentation")]
        let mut platform = match window {
            Some((title, width, height)) => {
                let mut platform =
                    bones_kernel_infra::platform::Platform::new(&title, width, height)
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
        // `exit-request` and `display-info`: what the extension host reads from
        // the kernel, offered as services so any module can read them the same
        // way (ADR-040).
        services
            .provide(exit_requested.clone())
            .expect("no other service registers as ExitRequest");
        services
            .provide(display_info.clone())
            .expect("no other service registers as DisplayInfo");
        services
            .provide(frame_control.clone())
            .expect("no other service registers as FrameControl");

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
        #[cfg(feature = "instance")]
        if let Some(primary) = self.instance.take() {
            let instance =
                bones_module_single_instance::InstanceModule::new(primary, self.logger.clone());
            register_module(
                &bus,
                &registry,
                &mut services,
                &mut modules,
                Box::new(instance),
            )
            .map_err(crate::Error::msg)?;
        }
        // Registered only when asked for: an unset variable leaves no
        // endpoint behind, not even an idle one.
        #[cfg(feature = "devtools")]
        if let Some(variable) = self.devtools_variable.take() {
            if bones_module_devtools::Devtools::requested(&variable) {
                let mut devtools =
                    bones_module_devtools::Devtools::from_env(variable).logger(self.logger.clone());
                if let Some(history) = devtools_logs {
                    devtools = devtools.log_history(history);
                }
                if let Some(redact) = self.devtools_redact.take() {
                    devtools = devtools.redacting(redact);
                }
                register_module(
                    &bus,
                    &registry,
                    &mut services,
                    &mut modules,
                    Box::new(devtools),
                )
                .map_err(crate::Error::msg)?;
            }
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

        // Registered last, after persistence and files: an extension's own
        // `init` may `send` any module synchronously (ADR-010), and that only
        // reaches it once it is registered. Its endpoint, `extension-manager`,
        // is where runtime commands arrive (ADR-040). Registered only when
        // the composition configures extensions, so an engine built with the
        // feature but asking for none carries no idle endpoint.
        #[cfg(feature = "wasm")]
        if self.extensions_dir.is_some()
            || !self.catalog_extensions.is_empty()
            || !self.startup_extensions.is_empty()
            || !self.required_extensions.is_empty()
            || self.extension_controller.is_some()
        {
            let mut wasm = bones_module_wasm::Wasm::new(self.logger.clone())
                .startup_extensions(self.startup_extensions.drain(..))
                .required_extensions(self.required_extensions.drain(..))
                .budget(self.extension_budget)
                .timeouts(self.extension_timeouts);
            for (name, permissions) in self.extension_permissions.drain(..) {
                wasm = wasm.permissions(name, permissions);
            }
            if let Some(dir) = &self.extensions_dir {
                wasm = wasm.extensions_dir(resolve_relative_to_exe(dir.clone()));
            }
            for (name, path) in self.catalog_extensions.drain(..) {
                wasm = wasm.catalog_extension(name, resolve_relative_to_exe(path));
            }
            if let Some(controller) = self.extension_controller.take() {
                wasm = wasm.controller(controller);
            }
            register_module(&bus, &registry, &mut services, &mut modules, Box::new(wasm))
                .map_err(crate::Error::msg)?;
        }

        #[cfg(feature = "presentation")]
        if let Some(platform) = &mut platform {
            platform.reclaim_window(&mut services);
        }

        Ok(BuiltKernel {
            runner: Runner::new(bus, self.logger),
            #[cfg(feature = "presentation")]
            platform,
            modules,
            registry,
            exit_requested,
            frame_control,
            shutdown_started: false,
        })
    }

    /// Runs at `tick_hz`, real wall-clock timing, until the window is
    /// closed (or forever, if `.window(...)` was never set) — a thin
    /// wrapper around `Runner::step` (ADR-014) that also runs every module's
    /// `before_tick` and `after_tick` around it, which is where the extension
    /// host applies reloads and quarantines faults (ADR-040).
    pub fn run(self) -> crate::Result<()> {
        let period = tick_period(self.tick_hz)?;
        let BuiltKernel {
            runner,
            #[cfg(feature = "presentation")]
            mut platform,
            modules,
            registry: _,
            exit_requested,
            frame_control,
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
                    bones_kernel_infra::bus::offer_event(&modules, event)
                });
                if platform.quit_requested() {
                    break "platform";
                }
            }
            if exit_requested.is_requested() {
                break ENGINE_SENDER;
            }

            let now = std::time::Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            // Paused by `frame-control`, the frame still dispatches and
            // renders: requests such as a capture reach their modules, the
            // window stays live, and a capture shows the held frame. Only
            // the tick and its hooks stop.
            match frame_control.next_dt(dt, period.as_secs_f32()) {
                Some(dt) => step_frame(&runner, &modules, dt),
                None => {
                    runner.begin_frame();
                    runner.bus().dispatch();
                }
            }

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

        run_shutdown(&runner, &modules, shutdown_sender);
        Ok(())
    }
}

/// Whether the environment variable named `variable` is set to anything but
/// empty or `0`; `false` when no variable is named.
#[cfg(feature = "presentation")]
fn env_flag(variable: Option<&str>) -> bool {
    variable
        .and_then(|name| std::env::var(name).ok())
        .is_some_and(|value| !value.is_empty() && value != "0")
}
impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
