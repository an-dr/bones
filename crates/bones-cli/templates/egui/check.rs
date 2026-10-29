//! Runs the built extension headless and checks its widget panel.

use bones_test::messages::ui::{Clicked, Spec, Widget};
use bones_test::App;

fn app() -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    App::load(wasm).unwrap()
}

fn label(app: &App) -> String {
    let spec = app.published::<Spec>().pop().expect("no ui/spec published");
    match spec.widgets.first() {
        Some(Widget::Label { text }) => text.to_string(),
        other => panic!("expected a label first, got {other:?}"),
    }
}

#[test]
fn the_panel_starts_at_zero() {
    let mut app = app();
    app.step(1);
    assert_eq!(label(&app), "Clicked 0 times");
}

#[test]
fn a_click_counts() {
    let mut app = app();
    app.send(&Clicked { id: 1 }).step(2);
    assert_eq!(label(&app), "Clicked 1 times");
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
