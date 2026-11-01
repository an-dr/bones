//! Runs the built extension headless. Headless has no web panels, so this
//! checks that the extension survives without one and says why.

use bones_test::App;

#[test]
fn it_runs_and_explains_a_missing_web_module() {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    let mut app = App::load(wasm).unwrap();
    app.step(1);

    assert!(app.is_running());
    assert!(app.logs().iter().any(|line| line.contains("no web panels")));
}
