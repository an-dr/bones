//! The TypeScript example run through the real extension host.
//!
//! Ignored by default: it needs `npm ci && npm run build` in
//! examples/extensions/ts-demo first, which needs Node.js. CI builds it and
//! runs `cargo test -p bones-test --test typescript_guest -- --ignored`.

use std::path::PathBuf;
use std::time::Duration;

use bones_test::kernel::Kernel;
use bones_test::messages::gfx::DrawRect;
use bones_test::messages::input::KeyDown;
use bones_test::App;

fn app() -> App {
    let wasm = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/extensions/ts-demo/dist/ts_demo.wasm");
    // A JavaScript component carries its own JS engine: it loads slower,
    // and its first calls are slower than a Rust guest's.
    let kernel = Kernel::new()
        .extension_load_timeout(Duration::from_secs(30))
        .extension_call_timeout(Duration::from_millis(500));
    App::with_kernel(kernel, wasm).unwrap()
}

#[test]
#[ignore = "needs the ts-demo component built with Node.js"]
fn the_typescript_guest_draws_and_moves() {
    let mut app = app();
    app.step(1);
    let start = app.published::<DrawRect>().last().expect("no draw").x;

    app.send(&KeyDown { key: "Right" }).step(30);

    let end = app.published::<DrawRect>().last().unwrap().x;
    assert!(end > start, "the square stayed at x={start}, now x={end}");
    assert!(app
        .logs()
        .iter()
        .any(|line| line.contains("ready: move the square")));
}
