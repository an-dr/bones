//! The real renderer in an offscreen window, against the sprite-demo
//! fixture `cargo xtask test` builds. Runs with `--features render`.

#![cfg(feature = "render")]

use std::path::PathBuf;

use bones_test::messages::gfx::Clear;
use bones_test::App;

fn sprite_demo() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../examples/extensions/sprite-demo/target/wasm32-wasip2/release/sprite_demo.wasm",
    );
    assert!(path.is_file(), "build the fixture first: cargo xtask test");
    path
}

#[test]
fn a_screenshot_shows_the_drawn_frame_and_does_not_advance_it() {
    let mut app = App::rendered(sprite_demo(), 800, 600).unwrap();
    app.step(1);
    let clear = app.published::<Clear>()[0];
    let ticks_before = app.messages().len();

    let shot = app.screenshot().unwrap();
    assert_eq!((shot.width, shot.height), (800, 600));
    assert!(shot.png.starts_with(b"\x89PNG"));
    // The sprites sit away from the corners, which show the clear color.
    let background = [clear.r, clear.g, clear.b, 255];
    assert_eq!(shot.pixel(0, 0), Some(background));
    let drawn = (0..600)
        .flat_map(|y| (0..800).map(move |x| (x, y)))
        .filter(|&(x, y)| shot.pixel(x, y) != Some(background))
        .count();
    assert!(
        drawn > 5_000,
        "only {drawn} pixels differ from the clear color"
    );

    let again = app.screenshot().unwrap();
    assert_eq!(again, shot, "a screenshot must not move the simulation");
    let published = &app.messages()[ticks_before..];
    assert!(
        published.iter().all(|e| e.topic.starts_with("renderer/")),
        "only the capture and its reply happened: {:?}",
        published.iter().map(|e| &e.topic).collect::<Vec<_>>()
    );
}

#[test]
fn the_window_size_is_the_screenshot_size() {
    let mut app = App::rendered(sprite_demo(), 320, 200).unwrap();
    app.step(1);
    let shot = app.screenshot().unwrap();
    assert_eq!((shot.width, shot.height), (320, 200));
}

#[test]
fn a_headless_app_says_how_to_get_a_screenshot() {
    let mut app = App::load(sprite_demo()).unwrap();
    let error = app.screenshot().unwrap_err();
    assert!(error.contains("App::rendered"), "{error}");
}
