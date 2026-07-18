//! Integration tests exercising `Engine` against real prebuilt `.wasm`
//! extensions and (for some) a real SDL window. Slower and fixture-
//! dependent, unlike `cargo test -p runner --lib`'s fast, fixture-free
//! unit tests — kept in their own binary for that reason.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Envelope, Handler, Module, ModuleContext};
use logging::{Logger, RecordingSink};
use runner::{BuiltEngine, Engine};

// SDL can't create windows concurrently across threads even with the
// test-mode feature (which only lifts the main-thread-only check) —
// cargo runs tests in parallel by default, so tests that open a real
// window take this lock to never run concurrently with each other.
fn sdl_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// Built by extensions/hello/build.ps1 (see its README).
const HELLO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extensions/hello/target/wasm32-wasip2/release");
// Built by extensions/keyecho/build.ps1 (see its README).
const KEYECHO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extensions/keyecho/target/wasm32-wasip2/release");
// Built by extensions/sprite_demo/build.ps1 (see its README).
const SPRITE_DEMO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extensions/sprite_demo/target/wasm32-wasip2/release");
// Built by extensions/runaway_demo/build.ps1 (see its README).
const RUNAWAY_DEMO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extensions/runaway_demo/target/wasm32-wasip2/release");
// Built by extensions/audio_demo/build.ps1 (see its README).
const AUDIO_DEMO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extensions/audio_demo/target/wasm32-wasip2/release");

#[test]
fn build_discovers_loads_and_registers_a_real_extension() {
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    let BuiltEngine { runner, platform, renderer, .. } = Engine::new()
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

    let BuiltEngine { runner, .. } = Engine::new()
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
    // doesn't lift, and cargo test's worker threads aren't it. That exact
    // mechanism is already proven in isolation by core/platform's own test
    // suite. Here, publish the envelope directly to prove Engine's wiring
    // (window + extension + subscription) without pumping real SDL events.
    let _guard = sdl_test_lock().lock().unwrap();
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    let BuiltEngine { runner, platform, .. } = Engine::new()
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
fn a_mouse_down_envelope_reaches_an_extension_through_a_real_window() {
    // Same reasoning as the key-down test above: publish directly rather
    // than pumping a real SDL mouse event — that mechanism is proven in
    // isolation by core/platform's own tests.
    let _guard = sdl_test_lock().lock().unwrap();
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    let BuiltEngine { runner, platform, .. } = Engine::new()
        .extensions_dir(KEYECHO_DIR)
        .logger(logger)
        .window("test", 64, 64)
        .build()
        .expect("build extensions/keyecho first: pwsh extensions/keyecho/build.ps1");
    assert!(platform.is_some(), ".window() was set");

    runner.bus().publish(Envelope {
        topic: "input/mouse-down".to_string(),
        sender: "platform".to_string(),
        correlation: None,
        payload: bones_messages::input::MouseDown { button: 1, x: 10.0, y: 20.0 }.encode(),
    });
    runner.step(1.0 / 60.0);

    let records = sink.records();
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("mouse button 1 pressed at (10, 20)")),
        "expected keyecho to log the injected mouse-down, got {records:?}"
    );
}

#[test]
fn audio_demo_loads_plays_music_and_reacts_to_a_key_press_through_a_real_audio_module() {
    // No window needed — audio doesn't touch the window-surface service.
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    let BuiltEngine { runner, .. } = Engine::new()
        .extensions_dir(AUDIO_DEMO_DIR)
        .logger(logger)
        .module(audio::Audio::new())
        .build()
        .expect("build extensions/audio_demo first: pwsh extensions/audio_demo/build.ps1");

    runner.step(1.0 / 60.0);

    let records = sink.records();
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("init: loaded sfx + music")),
        "expected audio_demo's init log (sfx/music loaded, music started), got {records:?}"
    );
    assert!(
        records.iter().all(|(_, _, msg)| !msg.contains("faulted")),
        "audio_demo must not fault against a real audio module, got {records:?}"
    );

    // The demo's on_message plays the SFX on any KeyDown — proves the
    // audio/play-sound path runs end to end without crashing the engine.
    // core/audio has no logger (see its own doc comment), so this can only
    // assert the engine keeps running, not that kira actually played
    // anything — that's covered by core/audio's own unit tests instead.
    runner.bus().publish(Envelope {
        topic: "input/key-down".to_string(),
        sender: "platform".to_string(),
        correlation: None,
        payload: bones_messages::input::KeyDown { key: "Space" }.encode(),
    });
    runner.step(1.0 / 60.0);
}

#[test]
fn a_window_with_no_renderer_or_module_is_not_dropped_with_the_service_registry() {
    // window-surface is now seeded into the registry unconditionally
    // (ADR-017, so a .module()-injected renderer can consume it too, not
    // just .renderer()) — this proves an unclaimed one is handed back to
    // `platform` instead of being dropped (and closing the window) with
    // the registry it briefly lived in.
    let _guard = sdl_test_lock().lock().unwrap();
    let BuiltEngine { mut platform, .. } = Engine::new().window("test", 64, 64).build().unwrap();

    assert!(
        platform.as_mut().unwrap().take_window().is_some(),
        "the window should still be available on `platform` when nothing claimed it"
    );
}

#[test]
fn a_custom_module_can_consume_window_surface_without_renderer() {
    // The whole point of a generic service registry (ADR-017) over a
    // renderer-only shortcut: an embedder's own `.module(...)` replacement
    // renderer must be able to get the window the same way the built-in
    // one does, with no `.renderer()` call and no privileged access.
    let _guard = sdl_test_lock().lock().unwrap();

    struct WantsWindow(Arc<Mutex<bool>>);
    impl Handler for WantsWindow {
        fn handle(&mut self, _envelope: &Envelope) {}
    }
    impl Module for WantsWindow {
        fn name(&self) -> &str {
            "wants-window"
        }
        fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
            *self.0.lock().unwrap() = ctx.consume_service::<sdl3::video::Window>().is_some();
            Ok(())
        }
    }

    let got_window = Arc::new(Mutex::new(false));
    let BuiltEngine { .. } = Engine::new()
        .window("test", 64, 64)
        .module(WantsWindow(got_window.clone()))
        .build()
        .unwrap();

    assert!(*got_window.lock().unwrap(), "expected the custom module to consume window-surface");
}

#[test]
fn a_real_extension_draws_a_sprite_through_a_real_renderer() {
    let _guard = sdl_test_lock().lock().unwrap();
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    let BuiltEngine { runner, renderer, .. } = Engine::new()
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
    renderer.lock().unwrap().render();
    renderer.lock().unwrap().present();
    runner.step(1.0 / 60.0);
    renderer.lock().unwrap().render();
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

    let BuiltEngine {
        runner,
        mut supervisor,
        ..
    } = Engine::new().extensions_dir(&dir).logger(logger).build().unwrap();
    let watcher = runner.bus().register("watcher", move |e: &Envelope| {
        sink_events.lock().unwrap().push(e.clone());
    });
    watcher.subscribe(LifecycleEvent::TOPIC);

    // hello ticks fine; runaway_demo hangs for its whole time budget
    // (~50ms) before trapping — this call blocks that long, same as a real
    // runaway extension would stall one frame before the supervisor
    // catches it.
    runner.step(1.0 / 60.0);
    supervisor.check();

    // A quarantined extension must be unreachable by direct send too, not
    // just off the bus -- otherwise Registry::call succeeds with a silent
    // empty reply instead of the error messaging.md promises.
    assert_eq!(
        supervisor.registry.call("test", "runaway_demo", b"hi"),
        Err(bus::SendError::UnknownEndpoint),
        "a quarantined extension must be unreachable via direct send"
    );

    // A second step must not hang again (quarantined, not called into) and
    // hello must still be ticking normally.
    runner.step(1.0 / 60.0);

    std::fs::remove_dir_all(&dir).ok();

    let records = sink.records();
    let hello_ticks = records
        .iter()
        .filter(|(_, category, msg)| category == "hello" && msg.contains("tick"))
        .count();
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
        .map(|e| {
            let message = LifecycleEvent::decode(&e.payload).unwrap();
            (message.event, message.extension.to_string())
        })
        .collect();
    assert!(
        events.contains(&(Event::Faulted, "runaway_demo".to_string())),
        "expected a Faulted lifecycle event, got {events:?}"
    );
}

/// Records every `Module`/`Handler` call it receives — proves `.module()`
/// runs the same init-then-subscribe-then-hook sequence a real module
/// (`renderer`) goes through, without needing SDL or a wasm fixture.
#[derive(Clone, Default)]
struct RecordingModule(Arc<Mutex<Vec<String>>>);

impl RecordingModule {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl Handler for RecordingModule {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().push(format!("handle:{}", envelope.topic));
    }
}

impl Module for RecordingModule {
    fn name(&self) -> &str {
        "recording"
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        self.0.lock().unwrap().push("init".to_string());
        ctx.subscribe("test/topic");
        Ok(())
    }

    fn render(&mut self) {
        self.0.lock().unwrap().push("render".to_string());
    }

    fn present(&mut self) {
        self.0.lock().unwrap().push("present".to_string());
    }
}

#[test]
fn a_custom_module_is_initialized_subscribed_and_hooked() {
    let module = RecordingModule::default();
    let BuiltEngine { runner, modules, .. } = Engine::new().module(module.clone()).build().unwrap();

    assert_eq!(modules.len(), 1, "expected the custom module to be registered");
    assert_eq!(module.calls(), vec!["init"], "init should run at build time, before any message or hook");

    runner.bus().publish(Envelope {
        topic: "test/topic".to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload: Vec::new(),
    });
    runner.step(1.0 / 60.0);

    modules[0].lock().unwrap().render();
    modules[0].lock().unwrap().present();

    assert_eq!(
        module.calls(),
        vec!["init", "handle:test/topic", "render", "present"],
        "expected the subscription requested in init to be applied, and render/present to reach the same instance"
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

    let BuiltEngine {
        runner,
        mut supervisor,
        ..
    } = Engine::new().extensions_dir(&dir).logger(logger).build().unwrap();
    let watcher = runner.bus().register("watcher", move |e: &Envelope| {
        sink_events.lock().unwrap().push(e.clone());
    });
    watcher.subscribe(LifecycleEvent::TOPIC);
    runner.step(1.0 / 60.0);

    // "Edit" the file: same bytes, but a deliberately later mtime so the
    // supervisor's polling notices — a real edit's mtime is real-clock-
    // later too, this just avoids a flaky sleep in the test.
    let original_mtime = std::fs::metadata(&wasm_path).unwrap().modified().unwrap();
    let file = std::fs::File::options().write(true).open(&wasm_path).unwrap();
    file.set_modified(original_mtime + Duration::from_secs(1)).unwrap();
    drop(file);

    supervisor.check();
    runner.step(1.0 / 60.0);

    std::fs::remove_dir_all(&dir).ok();

    let records = sink.records();
    let init_count = records
        .iter()
        .filter(|(_, category, msg)| category == "level" && msg.contains("init"))
        .count();
    assert_eq!(init_count, 2, "expected init to run again for the reloaded instance, got {records:?}");
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("reloaded 'level'")),
        "expected a reload confirmation, got {records:?}"
    );

    let events: Vec<_> = bus_events
        .lock()
        .unwrap()
        .iter()
        .map(|e| {
            let message = LifecycleEvent::decode(&e.payload).unwrap();
            (message.event, message.extension.to_string())
        })
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
