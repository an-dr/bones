use super::*;
use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Mod};

#[test]
fn escape_is_buffered_for_egui_but_never_consumed_from_game_input() {
    let mut ui = Ui::new(Bus::new(), Logger::default());
    let event = Event::KeyDown {
        timestamp: 0,
        window_id: 0,
        keycode: Some(Keycode::Escape),
        scancode: None,
        keymod: Mod::empty(),
        repeat: false,
        which: 0,
        raw: 0,
    };

    assert!(!ui.feed_event(&event));
    assert!(matches!(
        ui.events.as_slice(),
        [egui::Event::Key {
            key: egui::Key::Escape,
            pressed: true,
            ..
        }]
    ));
}
