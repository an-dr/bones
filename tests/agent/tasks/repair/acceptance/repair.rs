//! Hidden acceptance test for the repair task.

use bones_test::messages::gfx::DrawRect;
use bones_test::messages::input::KeyDown;
use bones_test::App;

fn app() -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    App::load(wasm).unwrap()
}

fn square(rects: &[DrawRect]) -> DrawRect {
    *rects
        .iter()
        .find(|r| r.w == 40 && r.h == 40)
        .expect("no 40 x 40 square drawn")
}

#[test]
fn it_keeps_running_and_drawing_with_a_bounded_trail() {
    let mut app = app();
    for frame in 0..200 {
        app.clear();
        app.step(1);
        assert!(app.is_running(), "quarantined at frame {frame}: {:?}", app.logs());
        let rects = app.published::<DrawRect>();
        square(&rects);
        assert!(rects.len() <= 101, "{} rectangles at frame {frame}", rects.len());
    }
}

#[test]
fn the_right_arrow_moves_the_square() {
    let mut app = app();
    app.step(1);
    let start = square(&app.published::<DrawRect>()).x;
    app.send(&KeyDown { key: "Right" }).step(30);
    app.clear();
    app.step(1);
    let end = square(&app.published::<DrawRect>()).x;
    assert!(end > start, "the square stayed at x={start}, now x={end}");
}
