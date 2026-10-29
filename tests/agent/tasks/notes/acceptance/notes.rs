//! Hidden acceptance test for the notes task.

use std::path::PathBuf;

use bones_test::kernel::Kernel;
use bones_test::messages::ui::{Changed, Clicked, Spec, Widget};
use bones_test::App;

/// An app saving into its own folder, so runs do not see each other.
fn app(saves: &PathBuf) -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    App::with_kernel(Kernel::new().saves_dir(saves), wasm).unwrap()
}

fn saves(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-bench-notes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The text edit's text and the labels of the latest spec.
fn panel(app: &mut App) -> (String, Vec<String>) {
    app.clear();
    app.step(1);
    let spec = app.published::<Spec>().pop().expect("no ui/spec published");
    assert_eq!(spec.title, "notes");
    let mut typed = None;
    let mut labels = Vec::new();
    for widget in &spec.widgets {
        match widget {
            Widget::TextEdit { id: 1, text } => typed = Some(text.to_string()),
            Widget::Label { text } => labels.push(text.to_string()),
            _ => {}
        }
    }
    assert!(
        spec.widgets.iter().any(|w| matches!(w, Widget::Button { id: 2, label } if *label == "Add")),
        "no Add button with id 2"
    );
    (typed.expect("no text edit with id 1"), labels)
}

#[test]
fn it_starts_empty() {
    let dir = saves("empty");
    let mut app = app(&dir);
    assert_eq!(panel(&mut app), (String::new(), Vec::new()));
}

#[test]
fn a_typed_note_is_added_and_the_field_clears() {
    let dir = saves("add");
    let mut app = app(&dir);
    app.send(&Changed { id: 1, text: "milk" }).step(1);
    app.send(&Clicked { id: 2 }).step(1);
    assert_eq!(panel(&mut app), (String::new(), vec!["milk".to_string()]));
}

#[test]
fn adding_nothing_changes_nothing() {
    let dir = saves("nothing");
    let mut app = app(&dir);
    app.send(&Clicked { id: 2 }).step(1);
    assert_eq!(panel(&mut app).1, Vec::<String>::new());
}

#[test]
fn notes_survive_a_restart() {
    let dir = saves("restart");
    {
        let mut app = app(&dir);
        for note in ["milk", "eggs"] {
            app.send(&Changed { id: 1, text: note }).step(1);
            app.send(&Clicked { id: 2 }).step(1);
        }
    }
    let mut app = app(&dir);
    assert_eq!(panel(&mut app).1, ["milk", "eggs"]);
}
