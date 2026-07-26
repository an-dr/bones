use bones_messages::gfx::{DrawRect, DrawText, TextAlign};
use bones_messages::{DecodeMessage, Message};

use super::*;

fn menu() -> VerticalMenu {
    VerticalMenu {
        canvas: Canvas::new(800, 600),
        panel_width: 420,
        header_height: 104,
        padding: 28,
        button_height: 44,
        gap: 10,
    }
}

#[test]
fn vertical_layout_centers_the_panel_and_buttons() {
    let layout = menu().layout([
        Button::new(1, "Start"),
        Button::new(2, "Settings"),
        Button::new(3, "Quit"),
    ]);
    assert_eq!(layout.panel.x, 190);
    assert_eq!(layout.buttons.len(), 3);
    assert!(layout.buttons.iter().all(|button| {
        button.bounds.x >= layout.panel.x
            && button.bounds.y >= layout.panel.y
            && button.bounds.x + button.bounds.width as i32
                <= layout.panel.x + layout.panel.width as i32
    }));
}

#[test]
fn excessive_padding_produces_zero_width_buttons_without_panicking() {
    let mut geometry = menu();
    geometry.padding = geometry.panel_width;
    let layout = geometry.layout([Button::new(1, "Still safe")]);
    assert_eq!(layout.buttons[0].bounds.width, 0);
}

#[test]
fn hit_testing_scales_physical_pixels_and_hover_updates_selection() {
    let layout = menu().layout([Button::new(10, "One"), Button::new(11, "Two")]);
    let target = layout.buttons[1].bounds;
    let mut selection = Selection::default();
    assert_eq!(
        selection.hover(
            &layout,
            Canvas::new(800, 600),
            (target.x as f32 + 2.0) * 2.0,
            (target.y as f32 + 2.0) * 2.0,
            (1600, 1200),
        ),
        Some(11)
    );
    assert_eq!(selection.index(), 1);
}

#[test]
fn selection_wraps_and_handles_empty_layouts() {
    let mut selection = Selection::default();
    selection.move_by(3, -1);
    assert_eq!(selection.index(), 2);
    selection.move_by(3, 1);
    assert_eq!(selection.index(), 0);
    selection.move_by(0, 1);
    assert_eq!(selection.index(), 0);
}

#[test]
fn draw_commands_publish_screen_space_gfx_messages() {
    let commands = [
        DrawCommand::rectangle(
            Rect {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
            true,
            (1, 2, 3, 4),
            9,
        ),
        DrawCommand::text("Hello", 5, 6, 16, (5, 6, 7, 8), 10),
        DrawCommand::text_aligned(
            "Centered",
            400,
            20,
            24,
            (9, 10, 11, 12),
            11,
            TextAlign::Center,
        ),
    ];
    let mut messages = Vec::new();
    for command in commands {
        command.publish_with(|topic, payload| messages.push((topic.to_owned(), payload.to_vec())));
    }
    assert!(DrawRect::decode(&messages[0].1)
        .is_ok_and(|message| messages[0].0 == DrawRect::TOPIC && message.screen_space));
    assert!(DrawText::decode(&messages[1].1).is_ok_and(|message| {
        messages[1].0 == DrawText::TOPIC && message.screen_space && message.align == TextAlign::Left
    }));
    assert!(DrawText::decode(&messages[2].1)
        .is_ok_and(|message| message.x == 400 && message.align == TextAlign::Center));
}
