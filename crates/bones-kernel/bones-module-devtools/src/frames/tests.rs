use bones_messages::gfx::Clear;
use bones_messages::{EncodeMessage, Message};

use super::*;

fn clear(sender: &str, r: u8) -> Envelope {
    Envelope {
        topic: Clear::TOPIC.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: Clear {
            r,
            g: 0,
            b: 0,
            a: 255,
        }
        .encode(),
    }
}

#[test]
fn a_frame_shows_once_its_tick_ends_it() {
    let mut frames = Frames::new();
    frames.record(&clear("pong", 1));
    assert!(frames.last().lock().unwrap().is_empty());

    frames.end_frame();

    let text = render(&frames.last().lock().unwrap());
    assert!(text.starts_with("[pong]\n"), "{text}");
    assert!(text.contains("Clear"), "{text}");
}

#[test]
fn a_sender_that_draws_nothing_keeps_its_last_batch() {
    let mut frames = Frames::new();
    frames.record(&clear("pong", 1));
    frames.end_frame();
    frames.record(&clear("hud", 2));
    frames.end_frame();

    let last = frames.last();
    let last = last.lock().unwrap();
    assert_eq!(last.len(), 2);
}

#[test]
fn a_malformed_draw_is_shown_with_its_error() {
    let mut frames = Frames::new();
    frames.record(&Envelope {
        payload: vec![1],
        ..clear("pong", 1)
    });
    frames.end_frame();

    let text = render(&frames.last().lock().unwrap());
    assert!(text.contains("does not decode"), "{text}");
}
