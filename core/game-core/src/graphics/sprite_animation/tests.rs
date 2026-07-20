use super::*;

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
