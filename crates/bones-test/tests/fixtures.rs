//! The harness against the example extensions `cargo xtask test` builds as
//! fixtures.

use std::path::PathBuf;

use bones_test::kernel::bus::BudgetLimits;
use bones_test::kernel::Kernel;
use bones_test::messages::gfx::DrawSprite;
use bones_test::messages::window::CloseRequested;
use bones_test::App;

fn fixture(example: &str, stem: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/extensions")
        .join(example)
        .join("target/wasm32-wasip2/release")
        .join(format!("{stem}.wasm"));
    assert!(
        path.is_file(),
        "build the fixture first: cargo xtask test, or cargo build --release --target wasm32-wasip2 in examples/extensions/{example}"
    );
    path
}

#[test]
fn a_drawing_extension_shows_its_frame_as_text() {
    let mut app = App::load(fixture("sprite-demo", "sprite_demo")).unwrap();
    app.step(1);

    let frame = app.frame_text();
    assert!(frame.starts_with("[sprite_demo]\n"), "{frame}");
    assert!(frame.contains("Clear(Clear {"), "{frame}");
    assert!(frame.contains("DrawSprite(DrawSprite {"), "{frame}");
    assert_eq!(app.published::<DrawSprite>().len(), 2);
}

#[test]
fn the_same_steps_draw_the_same_frame() {
    let path = fixture("sprite-demo", "sprite_demo");
    let mut first = App::load(&path).unwrap();
    let mut second = App::load(&path).unwrap();
    first.step(5);
    second.step(5);
    assert_eq!(first.frame_text(), second.frame_text());
}

#[test]
fn a_sent_message_reaches_the_extension_and_its_reply_is_recorded() {
    let mut app = App::load(fixture("hello-demo", "hello_demo")).unwrap();
    app.send(&CloseRequested).step(1);

    assert!(app
        .messages()
        .iter()
        .any(|envelope| envelope.topic == "hello/received" && envelope.sender == "hello_demo"));
}

#[test]
fn a_flooding_extension_is_quarantined_and_the_log_says_why() {
    // flood-demo publishes 64 messages a frame; the default allows 1024.
    let kernel = Kernel::new().extension_budget(BudgetLimits {
        max_inbound: 1024,
        max_publishes: 8,
    });
    let mut app = App::with_kernel(kernel, fixture("flood-demo", "flood_demo")).unwrap();
    app.step(3);

    assert!(!app.is_running());
    let logs = app.logs().join("\n");
    assert!(logs.contains("extension_max_publishes"), "{logs}");
}

#[test]
fn a_missing_file_says_to_build_it() {
    let error = App::load("no/such/extension.wasm").err().unwrap();
    assert!(error.contains("build the extension first"), "{error}");
}

#[test]
fn a_direct_send_reaches_the_extension() {
    // metrics-demo answers {"get":"history"} with its recorded values.
    let mut app = App::load(fixture("metrics-demo", "metrics_demo")).unwrap();
    app.step(1);
    let reply = app.request(br#"{"get":"history","id":1}"#).unwrap();
    assert!(String::from_utf8(reply)
        .unwrap()
        .contains("\"kind\":\"history\""));
}

#[test]
fn each_loaded_app_starts_with_no_saved_state() {
    // persistence-demo counts up and saves once a second; a fresh App must
    // load "nothing saved" whatever ran before it.
    let path = fixture("persistence-demo", "persistence_demo");
    let mut first = App::load(&path).unwrap();
    first.step(130);
    drop(first);
    let second = App::load(&path).unwrap();
    assert!(
        second
            .logs()
            .iter()
            .any(|line| line.contains("loaded counter = 0")),
        "{:?}",
        second.logs()
    );
}
