use super::*;

#[test]
fn a_running_control_simulates_with_the_measured_time() {
    let control = FrameControl::new();
    assert_eq!(control.next_dt(0.02, 0.016), Some(0.02));
}

#[test]
fn a_paused_control_skips_frames_until_asked_to_step() {
    let control = FrameControl::new();
    control.pause();
    assert_eq!(control.next_dt(0.02, 0.016), None);

    control.step(2);
    assert_eq!(control.next_dt(0.5, 0.016), Some(0.016));
    assert_eq!(control.next_dt(0.5, 0.016), Some(0.016));
    assert_eq!(control.next_dt(0.5, 0.016), None);
    assert_eq!(control.pending_steps(), 0);
}

#[test]
fn resuming_drops_steps_not_taken() {
    let control = FrameControl::new();
    control.pause();
    control.step(5);
    control.resume();
    assert!(!control.is_paused());
    assert_eq!(control.pending_steps(), 0);
}

#[test]
fn clones_share_one_control() {
    let control = FrameControl::new();
    control.clone().pause();
    assert!(control.is_paused());
}
