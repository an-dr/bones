//! The public builder API (design/modules.md): discovers WASM extensions
//! and runs them. TODO: `bones::Engine` in the design sketch — lives here
//! as `runner::Engine` until a top-level facade crate exists to re-export
//! it. TODO: no `.module(...)` yet for injecting custom native modules —
//! renderer is wired directly into this crate instead of through one.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bus::{Envelope, Handler, Registry};
use lifecycle::Event;
use logging::Logger;
use renderer::Renderer;

use crate::loading::{attach_extension, derive_extension_name, find_wasm_files, is_first_occurrence, read_file_mtime, ENGINE_SENDER};
use crate::supervisor::TrackedExtension;
use crate::Runner;
use crate::Supervisor;

const DEFAULT_TICK_HZ: f64 = 60.0;
const GFX_TOPICS: &str = "gfx/*";

/// Forwards bus deliveries to a `Renderer` shared with `Engine` itself (for
/// the `present()` call each frame, outside normal message delivery).
struct SharedRenderer(Arc<Mutex<Renderer>>);

impl Handler for SharedRenderer {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

/// Everything `Engine::build` wires up: the step-driven `Runner`, the
/// platform window (if `.window(...)` was set), the renderer (if
/// `.renderer()` was set), and the `Supervisor` sweeping loaded extensions
/// for faults and file changes.
pub struct BuiltEngine {
    pub runner: Runner,
    pub platform: Option<platform::Platform>,
    pub renderer: Option<Arc<Mutex<Renderer>>>,
    pub supervisor: Supervisor,
}

pub struct Engine {
    extensions_dir: Option<PathBuf>,
    logger: Logger,
    tick_hz: f64,
    window: Option<(String, u32, u32)>,
    renderer_enabled: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            extensions_dir: None,
            logger: Logger::default(),
            tick_hz: DEFAULT_TICK_HZ,
            window: None,
            renderer_enabled: false,
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

    /// Wires the bus, every `.wasm` file in `extensions_dir`, the window
    /// (if `.window(...)` was set), and the renderer (if `.renderer()` was
    /// set). A file that fails to load, or whose name is already taken, is
    /// logged and skipped rather than failing the whole engine. Exposed
    /// publicly (not just used by `run`) for a future driver that wants
    /// the wired-up pieces without `run`'s sleep-loop attached.
    pub fn build(mut self) -> wasmtime::Result<BuiltEngine> {
        let window = self.window.take();
        let renderer_enabled = self.renderer_enabled;
        let bus = bus::Bus::new();
        let registry = Registry::new();
        let wasm_engine = host::new_engine()?;
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

        let mut platform = match window {
            Some((title, width, height)) => {
                Some(platform::Platform::new(&title, width, height).map_err(wasmtime::Error::msg)?)
            }
            None => None,
        };

        let renderer = if renderer_enabled {
            let platform = platform
                .as_mut()
                .ok_or_else(|| wasmtime::Error::msg(".renderer() needs .window(...) too"))?;
            let window = platform
                .take_window()
                .ok_or_else(|| wasmtime::Error::msg("window already taken"))?;
            let shared = Arc::new(Mutex::new(Renderer::new(window, self.logger.clone())));
            let ep = bus.register("renderer", SharedRenderer(shared.clone()));
            ep.subscribe(GFX_TOPICS);
            Some(shared)
        } else {
            None
        };

        Ok(BuiltEngine {
            runner: Runner::new(bus, self.logger),
            platform,
            renderer,
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
            mut supervisor,
        } = self.build()?;

        let mut last = std::time::Instant::now() - period;
        loop {
            if let Some(platform) = &mut platform {
                platform.poll_events(runner.bus(), "platform");
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

            if let Some(renderer) = &renderer {
                renderer.lock().unwrap().present();
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
mod tests {
    use super::*;

    #[test]
    fn tick_hz_defaults_to_60_and_is_overridable() {
        assert_eq!(Engine::new().tick_hz, DEFAULT_TICK_HZ);
        assert_eq!(Engine::new().tick_hz(30.0).tick_hz, 30.0);
    }
}
