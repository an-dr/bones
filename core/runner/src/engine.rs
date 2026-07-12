//! The public builder API (design/modules.md): discovers WASM extensions
//! and runs them. TODO: `bones::Engine` in the design sketch — lives here
//! as `runner::Engine` until a top-level facade crate exists to re-export
//! it. TODO: no `.module(...)` yet for injecting custom native modules —
//! renderer is wired directly into this crate instead of through one.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bus::{Bus, Endpoint, Envelope, Handler, Registry, Respond};
use host::Host;
use lifecycle::Event;
use logging::Logger;
use renderer::Renderer;

use crate::Runner;

const DEFAULT_TICK_HZ: f64 = 60.0;
const GFX_TOPICS: &str = "gfx/*";
/// Publishes lifecycle events as this component (design/extensions.md).
const ENGINE_SENDER: &str = "engine";

/// Forwards bus deliveries to a `Renderer` shared with `Engine` itself (for
/// the `present()` call each frame, outside normal message delivery).
struct SharedRenderer(Arc<Mutex<Renderer>>);

impl Handler for SharedRenderer {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

/// One extension `Host`, shared between its `Bus` registration (pub/sub
/// delivery), its `Registry` registration (direct send, ADR-010), and the
/// `Supervisor` (which needs `Host::is_faulted` after every call to know
/// when to quarantine it) — all three need the same instance, not
/// independent copies, so state stays consistent across all of them.
#[derive(Clone)]
struct SharedHost(Arc<Mutex<Host>>);

impl Handler for SharedHost {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

impl Respond for SharedHost {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().respond(sender, payload)
    }
}

impl SharedHost {
    fn is_faulted(&self) -> bool {
        self.0.lock().unwrap().is_faulted()
    }
}

fn file_mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

struct TrackedExtension {
    name: String,
    path: PathBuf,
    mtime: SystemTime,
    endpoint: Endpoint,
    shared: SharedHost,
    /// Set once quarantined, so a later sweep doesn't re-log/re-publish for
    /// an already-handled fault every check forever.
    quarantined: bool,
}

/// Watches every loaded extension for a fault (ADR-007's time/queue
/// budgets, enforced inside `Host`) or a changed file
/// (design/extensions.md's Reloading state; the roadmap's "edit a level
/// extension, watch it swap in" demo), and reacts: quarantines a
/// newly-faulted one (drops its bus registration, publishes `Faulted`) or
/// hot-swaps a changed one in place (drop old, load new, publish
/// `Reloading`/`Reloaded`). The engine, and every other extension, keeps
/// running regardless.
pub struct Supervisor {
    wasm_engine: wasmtime::Engine,
    bus: Bus,
    registry: Registry,
    logger: Logger,
    tracked: Mutex<Vec<TrackedExtension>>,
}

impl Supervisor {
    /// Checks every tracked extension once: quarantines it if newly
    /// faulted, then (regardless) reloads it if its file changed since —
    /// a file change after a fault is exactly the "user fixed it and
    /// rebuilt" case, and reuses this same path rather than a separate
    /// explicit-reload mechanism. A replacement that fails to load is
    /// logged and the *old* instance (or nothing, if just quarantined)
    /// keeps running — a broken edit never takes an extension further
    /// down (ADR-007's "the engine itself never dies" spirit).
    pub fn check(&self) {
        let mut tracked = self.tracked.lock().unwrap();
        for extension in tracked.iter_mut() {
            if !extension.quarantined && extension.shared.is_faulted() {
                self.bus.unregister(&extension.endpoint);
                extension.quarantined = true;
                self.logger
                    .error("engine", &format!("'{}' faulted and was quarantined", extension.name));
                lifecycle::publish(&self.bus, ENGINE_SENDER, &extension.name, Event::Faulted);
            }

            let mtime = file_mtime(&extension.path);
            if mtime <= extension.mtime {
                continue;
            }

            lifecycle::publish(&self.bus, ENGINE_SENDER, &extension.name, Event::Reloading);
            match Host::load(
                &self.wasm_engine,
                &extension.path.to_string_lossy(),
                &extension.name,
                self.bus.clone(),
                self.registry.clone(),
                self.logger.clone(),
            ) {
                Ok(mut new_host) => {
                    let topics = new_host.requested_topics();
                    let shared = SharedHost(Arc::new(Mutex::new(new_host)));

                    if !extension.quarantined {
                        self.bus.unregister(&extension.endpoint);
                    }
                    let ep = self.bus.register(extension.name.clone(), shared.clone());
                    for topic in &topics {
                        ep.subscribe(topic);
                    }
                    self.registry.insert(extension.name.clone(), Arc::new(shared.clone()));

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
                        &format!("reload of '{}' failed, keeping the running instance: {err}", extension.name),
                    );
                }
            }
        }
    }
}

pub struct Engine {
    extensions_dir: Option<PathBuf>,
    logger: Logger,
    tick_hz: f64,
    window: Option<(String, u32, u32)>,
    renderer_enabled: bool,
}

impl Engine {
    pub fn builder() -> Self {
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

    /// Wires the bus, every `.wasm` file found in `extensions_dir` (each
    /// registered as a bus endpoint named after its file stem and
    /// subscribed to whatever topics it requested via `subscribe` during
    /// its own `init` — opt-in, including `core/tick`, messaging.md), the
    /// window if `.window(...)` was set, and the renderer if `.renderer()`
    /// was set. A file that fails to load is logged and skipped — one bad
    /// extension never stops the others (or startup) from proceeding.
    /// Also returns a `Supervisor`: every loaded extension is tracked so
    /// `Supervisor::check` can quarantine one that faults (ADR-007) or
    /// hot-swap one whose file changes.
    /// Exposed publicly (not just used by `run`) since a future driver
    /// (platform's own vsync-paced loop) will want the wired-up pieces
    /// without this module's sleep-loop attached to them — `run` is
    /// exactly that: `build` plus timing.
    #[allow(clippy::type_complexity)]
    pub fn build(
        mut self,
    ) -> wasmtime::Result<(Runner, Option<platform::Platform>, Option<Arc<Mutex<Renderer>>>, Supervisor)> {
        let window = self.window.take();
        let renderer_enabled = self.renderer_enabled;
        let bus = bus::Bus::new();
        let registry = Registry::new();
        let wasm_engine = host::new_engine()?;
        let mut tracked = Vec::new();

        if let Some(dir) = &self.extensions_dir {
            for path in find_wasm_files(dir) {
                let name = derive_extension_name(&path);
                match Host::load(&wasm_engine, &path.to_string_lossy(), &name, bus.clone(), registry.clone(), self.logger.clone()) {
                    Ok(mut extension) => {
                        let topics = extension.requested_topics();
                        let shared = SharedHost(Arc::new(Mutex::new(extension)));
                        let ep = bus.register(name.clone(), shared.clone());
                        for topic in &topics {
                            ep.subscribe(topic);
                        }
                        registry.insert(name.clone(), Arc::new(shared.clone()));
                        self.logger.info(
                            "engine",
                            &format!("loaded '{name}' from {} (subscribed: {topics:?})", path.display()),
                        );
                        lifecycle::publish(&bus, ENGINE_SENDER, &name, Event::Loaded);
                        tracked.push(TrackedExtension {
                            name,
                            mtime: file_mtime(&path),
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

        let supervisor = Supervisor {
            wasm_engine,
            bus: bus.clone(),
            registry,
            logger: self.logger.clone(),
            tracked: Mutex::new(tracked),
        };

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

        Ok((Runner::new(bus, self.logger), platform, renderer, supervisor))
    }

    /// Runs forever at `tick_hz` until the process is killed. A thin
    /// wrapper around `Runner::step` (ADR-014's "run forever is a thin
    /// wrapper, not the primitive tests use") — real wall-clock timing,
    /// unlike the headless runner's own bounded/virtual-clock API.
    ///
    /// `dt` passed to `step` is the *measured* time since the previous
    /// tick, not the nominal period — if a tick runs long, extensions see
    /// that in `dt` rather than being told a fixed value that didn't happen.
    ///
    /// If `.window(...)` was set, every iteration polls it first (input
    /// events land on the bus before `step`'s dispatch), matching the
    /// frame-phase order in architecture.md: poll input, then dispatch/tick.
    /// `Supervisor::check` runs both before and after `step` — before, so a
    /// swapped-in extension's first tick already runs against the new
    /// code; after, so a fault from that same tick is quarantined this
    /// iteration rather than one later. `check` does both jobs (reload,
    /// quarantine) either time it's called; calling it twice just lets
    /// each keep its own ideal timing without the other compromising it.
    /// If `.renderer()` was set, presents once after `step`'s dispatch —
    /// gfx/* commands an extension publishes reactively during its own
    /// `on-tick` land one frame later (ADR-015's deferred dispatch), same
    /// as any other reactive publish; invisible at any real frame rate.
    pub fn run(self) -> wasmtime::Result<()> {
        let period = Duration::from_secs_f64(1.0 / self.tick_hz);
        let (runner, mut platform, renderer, supervisor) = self.build()?;

        let mut last = std::time::Instant::now() - period;
        loop {
            if let Some(platform) = &mut platform {
                platform.poll_events(runner.bus(), "platform");
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
    }
}

/// `.wasm` files directly inside `dir`, sorted for deterministic load order.
/// A missing directory is "no extensions," not an error.
fn find_wasm_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    files.sort();
    files
}

fn derive_extension_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::RecordingSink;
    use std::sync::{Arc, Mutex, OnceLock};

    // SDL can't create windows concurrently across threads even with the
    // test-mode feature (which only lifts the main-thread-only check) —
    // cargo runs tests in parallel by default, so tests that open a real
    // window take this lock to never run concurrently with each other.
    fn sdl_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    // Built by extensions/hello/build.ps1 (see its README).
    const HELLO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/hello/target/wasm32-wasip2/release"
    );
    // Built by extensions/keyecho/build.ps1 (see its README).
    const KEYECHO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/keyecho/target/wasm32-wasip2/release"
    );
    // Built by extensions/sprite_demo/build.ps1 (see its README).
    const SPRITE_DEMO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/sprite_demo/target/wasm32-wasip2/release"
    );
    // Built by extensions/runaway_demo/build.ps1 (see its README).
    const RUNAWAY_DEMO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/runaway_demo/target/wasm32-wasip2/release"
    );

    #[test]
    fn tick_hz_defaults_to_60_and_is_overridable() {
        assert_eq!(Engine::builder().tick_hz, DEFAULT_TICK_HZ);
        assert_eq!(Engine::builder().tick_hz(30.0).tick_hz, 30.0);
    }

    #[test]
    fn find_wasm_files_finds_only_wasm_extensions_sorted() {
        let files = find_wasm_files(Path::new(HELLO_DIR));
        assert!(
            files.iter().all(|f| f.extension().unwrap() == "wasm"),
            "expected only .wasm files, got {files:?}"
        );
        assert!(
            files.iter().any(|f| f.file_stem().unwrap() == "hello"),
            "expected hello.wasm in {files:?} — run extensions/hello/build.ps1 first"
        );
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted);
    }

    #[test]
    fn find_wasm_files_on_a_missing_directory_is_empty_not_an_error() {
        assert_eq!(find_wasm_files(Path::new("no/such/directory")), Vec::<PathBuf>::new());
    }

    #[test]
    fn derive_extension_name_is_the_file_stem() {
        assert_eq!(derive_extension_name(Path::new("/a/b/hello.wasm")), "hello");
    }

    #[test]
    fn build_discovers_loads_and_registers_a_real_extension() {
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let (runner, platform, renderer, _supervisor) = Engine::builder()
            .extensions_dir(HELLO_DIR)
            .logger(logger)
            .build()
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1");
        assert!(platform.is_none(), "no .window() was set");
        assert!(renderer.is_none(), "no .renderer() was set");

        runner.step(1.0 / 60.0);

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("loaded 'hello'")),
            "expected a load confirmation, got {records:?}"
        );
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("tick")),
            "expected the extension's tick log line, got {records:?}"
        );
    }

    #[test]
    fn build_skips_a_component_that_fails_to_load_without_failing_the_engine() {
        let dir = std::env::temp_dir().join("bones-engine-test-bad-extension");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.wasm"), b"not a real component").unwrap();

        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let (runner, _platform, _renderer, _supervisor) = Engine::builder()
            .extensions_dir(&dir)
            .logger(logger)
            .build()
            .expect("a bad component must be skipped, not fail the whole engine");
        runner.step(1.0 / 60.0);

        std::fs::remove_dir_all(&dir).ok();

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("failed to load")),
            "expected a load failure to be logged, got {records:?}"
        );
    }

    #[test]
    fn a_key_down_envelope_reaches_an_extension_through_a_real_window() {
        // Real SDL event *pumping* (Platform::poll_events) asserts on the
        // OS-level main thread inside SDL's own C code — a check test-mode
        // doesn't lift, and cargo test's worker threads aren't it. That
        // exact mechanism (inject_event + poll_events) is already proven in
        // isolation by core/platform's own test suite, the only place in
        // this workspace that calls poll_events in a #[test]. Here, publish
        // the envelope directly to prove Engine's wiring (window +
        // extension + subscription) without pumping real SDL events.
        let _guard = sdl_test_lock().lock().unwrap();
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let (runner, platform, _renderer, _supervisor) = Engine::builder()
            .extensions_dir(KEYECHO_DIR)
            .logger(logger)
            .window("test", 64, 64)
            .build()
            .expect("build extensions/keyecho first: pwsh extensions/keyecho/build.ps1");
        assert!(platform.is_some(), ".window() was set");

        runner.bus().publish(Envelope {
            topic: "input/key-down".to_string(),
            sender: "platform".to_string(),
            correlation: None,
            payload: b"A".to_vec(),
        });
        runner.step(1.0 / 60.0);

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("key pressed: A")),
            "expected keyecho to log the injected keypress, got {records:?}"
        );
    }

    #[test]
    fn a_real_extension_draws_a_sprite_through_a_real_renderer() {
        let _guard = sdl_test_lock().lock().unwrap();
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let (runner, _platform, renderer, _supervisor) = Engine::builder()
            .extensions_dir(SPRITE_DEMO_DIR)
            .logger(logger)
            .window("test", 400, 300)
            .renderer()
            .build()
            .expect("build extensions/sprite_demo first: pwsh extensions/sprite_demo/build.ps1");
        let renderer = renderer.expect(".renderer() was set");

        // First tick: gfx/load-sprite (queued during init) gets delivered.
        // Second: gfx/clear + gfx/draw-sprite (queued reactively from the
        // first tick's on-tick) get delivered — ADR-015's deferred dispatch.
        runner.step(1.0 / 60.0);
        renderer.lock().unwrap().present();
        runner.step(1.0 / 60.0);
        renderer.lock().unwrap().present();

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("sprite loaded")),
            "expected sprite_demo's init log, got {records:?}"
        );
        assert!(
            records.iter().all(|(_, category, _)| category != "renderer"),
            "expected no renderer errors (bad PNG decode or unknown sprite id), got {records:?}"
        );
    }

    #[test]
    fn a_runaway_extension_is_quarantined_while_the_engine_keeps_running() {
        let dir = std::env::temp_dir().join("bones-engine-test-runaway");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(format!("{HELLO_DIR}/hello.wasm"), dir.join("hello.wasm"))
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1");
        std::fs::copy(format!("{RUNAWAY_DEMO_DIR}/runaway_demo.wasm"), dir.join("runaway_demo.wasm"))
            .expect("build extensions/runaway_demo first: pwsh extensions/runaway_demo/build.ps1");

        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));
        let bus_events = Arc::new(Mutex::new(Vec::new()));
        let sink_events = bus_events.clone();

        let (runner, _platform, _renderer, supervisor) =
            Engine::builder().extensions_dir(&dir).logger(logger).build().unwrap();
        let watcher = runner.bus().register("watcher", move |e: &Envelope| {
            sink_events.lock().unwrap().push(e.clone());
        });
        watcher.subscribe(lifecycle::TOPIC);

        // hello ticks fine; runaway_demo hangs for its whole time budget
        // (~50ms) before trapping — this call blocks that long, same as a
        // real runaway extension would stall one frame before the
        // supervisor catches it.
        runner.step(1.0 / 60.0);
        supervisor.check();

        // A second step must not hang again (quarantined, not called into)
        // and hello must still be ticking normally.
        runner.step(1.0 / 60.0);

        std::fs::remove_dir_all(&dir).ok();

        let records = sink.records();
        let hello_ticks = records.iter().filter(|(_, _, msg)| msg.contains("hello extension: tick")).count();
        assert_eq!(hello_ticks, 2, "expected hello to keep ticking normally, got {records:?}");
        assert!(
            records
                .iter()
                .any(|(_, _, msg)| msg.contains("'runaway_demo' faulted and was quarantined")),
            "expected a quarantine log line, got {records:?}"
        );

        let events: Vec<_> = bus_events
            .lock()
            .unwrap()
            .iter()
            .map(|e| lifecycle::parse(&e.payload).unwrap())
            .collect();
        assert!(
            events.contains(&(Event::Faulted, "runaway_demo".to_string())),
            "expected a Faulted lifecycle event, got {events:?}"
        );
    }

    #[test]
    fn a_changed_wasm_file_is_hot_reloaded_in_place() {
        let dir = std::env::temp_dir().join("bones-engine-test-hot-reload");
        std::fs::create_dir_all(&dir).unwrap();
        let wasm_path = dir.join("level.wasm");
        std::fs::copy(format!("{HELLO_DIR}/hello.wasm"), &wasm_path)
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1");

        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));
        let bus_events = Arc::new(Mutex::new(Vec::new()));
        let sink_events = bus_events.clone();

        let (runner, _platform, _renderer, supervisor) = Engine::builder()
            .extensions_dir(&dir)
            .logger(logger)
            .build()
            .unwrap();
        let watcher = runner.bus().register("watcher", move |e: &Envelope| {
            sink_events.lock().unwrap().push(e.clone());
        });
        watcher.subscribe(lifecycle::TOPIC);
        runner.step(1.0 / 60.0);

        // "Edit" the file: same bytes, but a deliberately later mtime so
        // the supervisor's polling notices — a real edit's mtime is
        // real-clock-later too, this just avoids a flaky sleep in the test.
        let original_mtime = std::fs::metadata(&wasm_path).unwrap().modified().unwrap();
        let file = std::fs::File::options().write(true).open(&wasm_path).unwrap();
        file.set_modified(original_mtime + Duration::from_secs(1)).unwrap();
        drop(file);

        supervisor.check();
        runner.step(1.0 / 60.0);

        std::fs::remove_dir_all(&dir).ok();

        let records = sink.records();
        let init_count = records.iter().filter(|(_, _, msg)| msg.contains("hello extension: init")).count();
        assert_eq!(init_count, 2, "expected init to run again for the reloaded instance, got {records:?}");
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("reloaded 'level'")),
            "expected a reload confirmation, got {records:?}"
        );

        let events: Vec<_> = bus_events
            .lock()
            .unwrap()
            .iter()
            .map(|e| lifecycle::parse(&e.payload).unwrap())
            .collect();
        assert!(
            events.contains(&(Event::Loaded, "level".to_string())),
            "expected a Loaded lifecycle event, got {events:?}"
        );
        assert!(
            events.contains(&(Event::Reloading, "level".to_string())),
            "expected a Reloading lifecycle event, got {events:?}"
        );
        assert!(
            events.contains(&(Event::Reloaded, "level".to_string())),
            "expected a Reloaded lifecycle event, got {events:?}"
        );
    }
}
