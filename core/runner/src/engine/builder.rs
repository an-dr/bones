//! The public builder API (design/modules.md, ADR-017): discovers WASM
//! extensions and runs them, plus `.module(...)` for injecting custom
//! native modules. TODO: `bones::Engine` in the design sketch — lives here
//! as `runner::Engine` until a top-level facade crate exists to re-export
//! it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bones_messages::Message;
use bus::{Module, ModuleContext, Registry, ServiceRegistry};
use logging::Logger;
use renderer::Renderer;
use ui::Ui;
use wasm_extensions::lifecycle;
use wasm_extensions::lifecycle::Event;
use wasm_extensions::persistence::Persistence;

use crate::loading::{attach_extension, derive_extension_name, find_wasm_files, is_first_occurrence, read_file_mtime, ENGINE_SENDER};
use crate::supervisor::TrackedExtension;
use crate::Runner;
use crate::Supervisor;

use super::built_engine::BuiltEngine;
use super::register_module::register_module;
use super::shared::Shared;

const DEFAULT_TICK_HZ: f64 = 60.0;
const GFX_TOPICS: &str = "gfx/*";

pub struct Engine {
    extensions_dir: Option<PathBuf>,
    logger: Logger,
    tick_hz: f64,
    window: Option<(String, u32, u32)>,
    renderer_enabled: bool,
    ui_enabled: bool,
    modules: Vec<Box<dyn Module>>,
    saves_dir: PathBuf,
    persistence_read_only: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            extensions_dir: None,
            logger: Logger::default(),
            tick_hz: DEFAULT_TICK_HZ,
            window: None,
            renderer_enabled: false,
            ui_enabled: false,
            modules: Vec::new(),
            saves_dir: PathBuf::from("saves"),
            persistence_read_only: false,
        }
    }

    pub fn extensions_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.extensions_dir = Some(path.into());
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
    pub fn window(mut self, title: impl Into<String>, width: u32, height: u32) -> Self {
        self.window = Some((title.into(), width, height));
        self
    }

    /// Attaches a renderer to the window (design/modules.md, ADR-002):
    /// executes `gfx/*` draw commands published by extensions and presents
    /// once per `run` iteration. Requires `.window(...)` — `build`/`run`
    /// error if this is set without one.
    pub fn renderer(mut self) -> Self {
        self.renderer_enabled = true;
        self
    }

    /// Attaches the egui ui module (ADR-005, design/presentation.md):
    /// decodes `ui/spec` messages and draws them each `run` iteration.
    /// Requires `.renderer()` too (ui draws through it, direct-wired for
    /// now — see docs/structure.md) — `build`/`run` error if this is set
    /// without one.
    pub fn ui(mut self) -> Self {
        self.ui_enabled = true;
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
    /// process's cwd if not absolute — the same convention
    /// `extensions_dir` already uses.
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

    /// Wires the bus, every `.wasm` file in `extensions_dir`, the window
    /// (if `.window(...)` was set), and the renderer (if `.renderer()` was
    /// set). A file that fails to load, or whose name is already taken, is
    /// logged and skipped rather than failing the whole engine. Exposed
    /// publicly (not just used by `run`) for a future driver that wants
    /// the wired-up pieces without `run`'s sleep-loop attached.
    pub fn build(mut self) -> wasmtime::Result<BuiltEngine> {
        let window = self.window.take();
        let renderer_enabled = self.renderer_enabled;
        let ui_enabled = self.ui_enabled;
        let bus = bus::Bus::new();
        let registry = Registry::new();
        let wasm_engine = wasm_extensions::host::new_engine()?;

        let mut platform = match window {
            Some((title, width, height)) => {
                Some(platform::Platform::new(&title, width, height).map_err(wasmtime::Error::msg)?)
            }
            None => None,
        };

        // Build-time-only (ADR-017): every module's `init` runs against
        // this. Seeded with `window-surface` unconditionally (not just for
        // `.renderer()`) so a `.module(...)`-injected replacement renderer
        // can consume it too, same as the built-in one — no privileged
        // access (design/modules.md). Reclaimed by `platform` below if
        // nothing ends up consuming it, so an unclaimed window stays open
        // instead of closing with the registry it briefly lived in.
        let mut services = ServiceRegistry::new();
        if let Some(platform) = &mut platform {
            platform.provide_window(&mut services);
        }

        let renderer = if renderer_enabled {
            if platform.is_none() {
                return Err(wasmtime::Error::msg(".renderer() needs .window(...) too"));
            }
            let shared = Arc::new(Mutex::new(Renderer::new(self.logger.clone())));
            {
                let mut renderer = shared.lock().unwrap();
                let mut ctx = ModuleContext::new(&mut services);
                Module::init(&mut *renderer, &mut ctx).map_err(wasmtime::Error::msg)?;
            }
            let ep = bus.register("renderer", Shared(shared.clone()));
            ep.subscribe(GFX_TOPICS);
            Some(shared)
        } else {
            None
        };

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
        for module in self.modules.drain(..) {
            register_module(&bus, &registry, &mut services, &mut modules, module).map_err(wasmtime::Error::msg)?;
        }

        // Unconditional (persistence's own doc comment explains why) —
        // registered here, not through `self.modules`, so there's no
        // `.persistence()`-style opt-in to forget.
        let persistence = Persistence::new(self.saves_dir.clone(), self.persistence_read_only);
        register_module(&bus, &registry, &mut services, &mut modules, Box::new(persistence))
            .map_err(wasmtime::Error::msg)?;

        let mut tracked = Vec::new();
        let mut loaded_names = std::collections::HashSet::new();

        if let Some(dir) = &self.extensions_dir {
            for path in find_wasm_files(dir) {
                let name = derive_extension_name(&path);
                if !is_first_occurrence(&mut loaded_names, &name) {
                    self.logger.error(
                        "engine",
                        &format!("skipping {}: an extension named '{name}' is already loaded", path.display()),
                    );
                    continue;
                }
                match attach_extension(&wasm_engine, &bus, &registry, &self.logger, &path, &name) {
                    Ok((ep, shared, topics)) => {
                        self.logger.info(
                            "engine",
                            &format!("loaded '{name}' from {} (subscribed: {topics:?})", path.display()),
                        );
                        lifecycle::publish(&bus, ENGINE_SENDER, &name, Event::Loaded);
                        tracked.push(TrackedExtension {
                            name,
                            mtime: read_file_mtime(&path),
                            path,
                            endpoint: ep,
                            shared,
                            quarantined: false,
                        });
                    }
                    Err(err) => {
                        self.logger
                            .error("engine", &format!("failed to load {}: {err}", path.display()));
                        lifecycle::publish(&bus, ENGINE_SENDER, &name, Event::Faulted);
                    }
                }
            }
        }

        let supervisor = Supervisor::new(wasm_engine, bus.clone(), registry, self.logger.clone(), tracked);

        if let Some(platform) = &mut platform {
            platform.reclaim_window(&mut services);
        }

        Ok(BuiltEngine {
            runner: Runner::new(bus, self.logger),
            platform,
            renderer,
            ui,
            modules,
            supervisor,
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
            mut platform,
            renderer,
            ui,
            modules,
            mut supervisor,
        } = self.build()?;

        let mut last = std::time::Instant::now() - period;
        loop {
            if let Some(platform) = &mut platform {
                // ADR-008: offer every raw event to the ui layer first; what
                // it claims (wants_pointer_input/wants_keyboard_input, as of
                // the end of the last `update`) never reaches `input/*`.
                // Locked once for the whole poll, not per event.
                let mut ui_guard = ui.as_ref().map(|ui| ui.lock().unwrap());
                platform.poll_events_with(runner.bus(), "platform", |event| {
                    ui_guard.as_mut().is_some_and(|ui| ui.feed_event(event))
                });
                // Minimal shutdown slice: exit cleanly on a window close
                // request. TODO: no close-request-as-event or shutdown()
                // call to extensions yet (design/platform.md's full
                // sequence) — a future roadmap rung.
                if platform.quit_requested() {
                    break;
                }
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
            if let Some(renderer) = &renderer {
                renderer.lock().unwrap().render();
            }
            for module in &modules {
                module.lock().unwrap().render();
            }

            // ui draws above every gfx layer (design/presentation.md), so
            // its `update` runs between `render` and `present`.
            if let (Some(ui), Some(renderer)) = (&ui, &renderer) {
                let mut renderer = renderer.lock().unwrap();
                let (width, height) = renderer.size();
                ui.lock().unwrap().update(&mut renderer, width, height);
            }

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
        }

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
