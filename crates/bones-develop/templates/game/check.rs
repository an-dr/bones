//! Runs the built extension headless and checks what it draws.

use bones_test::messages::gfx::DrawRect;
use bones_test::messages::input::KeyDown;
use bones_test::App;

fn app() -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    App::load(wasm).unwrap()
}

#[test]
fn it_draws_the_square_every_frame() {
    let mut app = app();
    app.step(1);
    assert!(app.frame_text().contains("DrawRect"), "{}", app.frame_text());
}

#[test]
fn the_right_arrow_moves_the_square_right() {
    let mut app = app();
    app.step(1);
    let start = app.published::<DrawRect>().last().unwrap().x;

    app.send(&KeyDown { key: "Right" }).step(30);

    let end = app.published::<DrawRect>().last().unwrap().x;
    assert!(end > start, "the square stayed at x={start}, now x={end}");
}

#[test]
fn it_keeps_running_without_warnings() {
    let mut app = app();
    app.step(120);
    assert!(app.is_running(), "quarantined: {:?}", app.logs());
    let problems: Vec<String> = app
        .logs()
        .into_iter()
        .filter(|line| line.starts_with("WARN") || line.starts_with("ERROR"))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
}
