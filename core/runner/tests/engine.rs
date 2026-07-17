//! Integration tests exercising `Engine` against real prebuilt `.wasm`
//! extensions and (for some) a real SDL window. Slower and fixture-
//! dependent, unlike `cargo test -p runner --lib`'s fast, fixture-free
//! unit tests — kept in their own binary for that reason.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::{DecodeMessage, Message};
use bus::Envelope;
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
