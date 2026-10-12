use super::*;
use bones_messages::game_core::{Sprite, SpritePresentation};

fn presentation() -> SpritePresentation {
    SpritePresentation {
        sprite: Sprite {
            sprite_id: 7,
            frame_w: 64,
            frame_h: 64,
            frame_count: 5,
            frame_duration: 0.1,
        },
        frames_per_row: 4,
        draw_w: 128,
        draw_h: 96,
        looping: false,
        advance_while_stopped: true,
        flip_h: true,
        flip_v: false,
    }
}

#[test]
fn a_static_sprite_never_advances() {
    let mut anim = SpriteAnimation::new(1, 16, 16, 1, 0.1);
    anim.advance(10.0);
    assert_eq!(anim.current_frame(), 0);
    assert_eq!(anim.current_src_x(), 0);
}

#[test]
fn advancing_past_a_frame_duration_moves_to_the_next_frame() {
    let mut anim = SpriteAnimation::new(1, 16, 16, 4, 0.1);
    anim.advance(0.15);
    assert_eq!(anim.current_frame(), 1);
    assert_eq!(anim.current_src_x(), 16);
}

#[test]
fn the_loop_wraps_back_to_frame_zero() {
    let mut anim = SpriteAnimation::new(1, 16, 16, 4, 0.1);
    anim.advance(0.41);
    assert_eq!(anim.current_frame(), 0);
}

#[test]
fn a_nonpositive_frame_duration_never_advances() {
    let mut anim = SpriteAnimation::new(1, 16, 16, 4, 0.0);
    anim.advance(10.0);
    assert_eq!(anim.current_frame(), 0);
}

#[test]
fn a_grid_animation_moves_to_the_next_row() {
    let mut anim = SpriteAnimation::from_presentation(presentation());
    anim.advance(0.41);
    assert_eq!(anim.current_frame(), 4);
    assert_eq!((anim.current_src_x(), anim.current_src_y()), (0, 64));
}

#[test]
fn a_non_looping_animation_stays_on_its_final_frame() {
    let mut anim = SpriteAnimation::from_presentation(presentation());
    anim.advance(10.0);
    assert_eq!(anim.current_frame(), 4);
    assert!(anim.is_finished());
}

#[test]
fn a_presentation_carries_draw_and_mirroring_options() {
    let anim = SpriteAnimation::from_presentation(presentation());
    assert_eq!((anim.draw_w, anim.draw_h), (128, 96));
    assert!(anim.advance_while_stopped);
    assert!(anim.flip_h);
    assert!(!anim.flip_v);
}
