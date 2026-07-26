use bones_messages::gfx::DrawRect;

use super::*;

fn rectangle() -> RetainedDraw {
    RetainedDraw::Rect(DrawRect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
        filled: true,
        color: (255, 255, 255, 255),
        layer: 0,
        screen_space: true,
    })
}

#[test]
fn clearing_a_batch_replaces_only_the_publishing_senders_retained_draws() {
    let mut pending = HashMap::new();
    let mut retained = HashMap::new();
    let mut sender_order = vec!["menu".to_string(), "level".to_string()];
    push_pending_draw(&mut retained, "menu", rectangle());
    push_pending_draw(&mut retained, "level", rectangle());

    clear_pending_draws(&mut pending, "menu");
    retain_completed_batches(&mut pending, &mut retained, &mut sender_order);

    assert!(retained["menu"].is_empty());
    assert_eq!(retained["level"].len(), 1);
    assert_eq!(sender_order, ["menu", "level"]);
}

#[test]
fn draws_after_a_clear_start_the_senders_replacement_batch() {
    let mut pending = HashMap::new();
    push_pending_draw(&mut pending, "menu", rectangle());
    clear_pending_draws(&mut pending, "menu");
    push_pending_draw(&mut pending, "menu", rectangle());

    assert_eq!(pending["menu"].len(), 1);
}
