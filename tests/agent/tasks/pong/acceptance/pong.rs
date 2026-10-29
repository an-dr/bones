//! Hidden acceptance test for the pong task.

use bones_test::messages::gfx::{DrawRect, DrawText};
use bones_test::messages::input::{KeyDown, KeyUp};
use bones_test::App;

fn app() -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    App::load(wasm).unwrap()
}

/// The rectangles and text of one frame.
fn frame(app: &mut App) -> (Vec<DrawRect>, Vec<String>) {
    app.clear();
    app.step(1);
    let rects = app.published::<DrawRect>();
    let texts = app
        .published::<DrawText>()
        .iter()
        .map(|text| text.text.to_string())
        .collect();
    (rects, texts)
}

fn paddle(rects: &[DrawRect], left: bool) -> DrawRect {
    *rects
        .iter()
        .find(|r| r.w == 10 && r.h == 80 && (r.x < 400) == left)
        .expect("no paddle drawn")
}

fn ball(rects: &[DrawRect]) -> DrawRect {
    *rects
        .iter()
        .find(|r| r.w == 10 && r.h == 10)
        .expect("no ball drawn")
}

#[test]
fn a_frame_draws_two_paddles_a_ball_and_the_score() {
    let mut app = app();
    let (rects, texts) = frame(&mut app);
    assert_eq!(rects.len(), 3, "{rects:?}");
    assert_eq!(paddle(&rects, true).x, 20);
    assert_eq!(paddle(&rects, false).x, 770);
    ball(&rects);
    assert!(texts.iter().any(|t| t == "0 - 0"), "{texts:?}");
}

#[test]
fn the_ball_moves_right_and_down() {
    let mut app = app();
    let (rects, _) = frame(&mut app);
    let start = ball(&rects);
    app.step(10);
    let (rects, _) = frame(&mut app);
    let later = ball(&rects);
    assert!(later.x > start.x && later.y > start.y, "{start:?} -> {later:?}");
}

#[test]
fn w_and_s_move_the_left_paddle() {
    let mut app = app();
    let (rects, _) = frame(&mut app);
    let start = paddle(&rects, true).y;
    app.send(&KeyDown { key: "W" }).step(20);
    let up = paddle(&frame(&mut app).0, true).y;
    app.send(&KeyUp { key: "W" }).send(&KeyDown { key: "S" }).step(40);
    let down = paddle(&frame(&mut app).0, true).y;
    assert!(up < start, "W: {start} -> {up}");
    assert!(down > up, "S: {up} -> {down}");
}

#[test]
fn the_arrows_move_the_right_paddle() {
    let mut app = app();
    let start = paddle(&frame(&mut app).0, false).y;
    app.send(&KeyDown { key: "Down" }).step(20);
    let down = paddle(&frame(&mut app).0, false).y;
    assert!(down > start, "Down: {start} -> {down}");
}

#[test]
fn the_ball_stays_on_the_canvas_vertically() {
    let mut app = app();
    for _ in 0..300 {
        let (rects, _) = frame(&mut app);
        let ball = ball(&rects);
        assert!((-10..=600).contains(&ball.y), "ball left the canvas: {ball:?}");
    }
}

#[test]
fn a_missed_ball_scores() {
    let mut app = app();
    let mut texts = Vec::new();
    for _ in 0..900 {
        texts = frame(&mut app).1;
        if !texts.iter().any(|t| t == "0 - 0") {
            break;
        }
    }
    assert!(
        texts.iter().any(|t| t == "1 - 0" || t == "0 - 1"),
        "no point scored in 15 seconds: {texts:?}"
    );
}
