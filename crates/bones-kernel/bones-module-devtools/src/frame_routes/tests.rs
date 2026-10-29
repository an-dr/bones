use bones_kernel_infra::frame_control::FrameControl;
use bones_messages::renderer::Scope;

use crate::server::route;
use crate::tests::{request, state};

#[test]
fn pause_and_resume_drive_the_frame_control() {
    let control = FrameControl::new();
    let mut state = state();
    state.frame_control = Some(control.clone());

    route(&request("POST /pause HTTP/1.1", ""), &state);
    assert!(control.is_paused());
    route(&request("POST /resume HTTP/1.1", ""), &state);
    assert!(!control.is_paused());
}

#[test]
fn step_needs_a_pause_first() {
    let mut state = state();
    state.frame_control = Some(FrameControl::new());

    let response = route(&request("POST /step?frames=2 HTTP/1.1", ""), &state);

    assert_eq!(response.status, "409 Conflict");
}

#[test]
fn step_answers_once_the_loop_has_taken_every_frame() {
    let control = FrameControl::new();
    control.pause();
    let mut state = state();
    state.frame_control = Some(control.clone());
    // Stands in for the run loop, taking frames as they are asked for.
    let loop_control = control.clone();
    let frame_loop = std::thread::spawn(move || {
        let mut taken = 0;
        while taken < 3 {
            if loop_control.next_dt(0.0, 0.016).is_some() {
                taken += 1;
            }
            std::thread::yield_now();
        }
    });

    let response = route(&request("POST /step?frames=3 HTTP/1.1", ""), &state);

    frame_loop.join().unwrap();
    assert_eq!(response.status, "200 OK", "{}", response.text_body());
    assert_eq!(control.pending_steps(), 0);
}

#[test]
fn frame_routes_say_so_without_a_frame_control() {
    let response = route(&request("POST /pause HTTP/1.1", ""), &state());
    assert_eq!(response.status, "404 Not Found");
}

#[test]
fn frame_lists_each_senders_batch() {
    let state = state();
    state
        .frames
        .lock()
        .unwrap()
        .insert("pong".to_string(), vec!["Clear(..)".to_string()]);

    let response = route(&request("GET /frame HTTP/1.1", ""), &state);

    assert_eq!(response.text_body(), "[pong]\nClear(..)\n");
}

#[test]
fn a_capture_already_answered_is_returned_as_png() {
    let mut state = state();
    state.bus = Some(bones_kernel_infra::bus::Bus::new());
    state
        .captures
        .lock()
        .unwrap()
        .insert(0, (Scope::Canvas, b"\x89PNG".to_vec()));

    let response = route(&request("GET /screenshot HTTP/1.1", ""), &state);

    assert_eq!(response.content_type, "image/png");
    assert_eq!(response.body, b"\x89PNG");
    assert_eq!(response.headers, [("X-Bones-Capture-Scope", "canvas")]);
}

#[test]
fn a_window_request_answered_with_the_canvas_says_so() {
    let mut state = state();
    state.bus = Some(bones_kernel_infra::bus::Bus::new());
    state
        .captures
        .lock()
        .unwrap()
        .insert(0, (Scope::Canvas, b"\x89PNG".to_vec()));

    let response = route(
        &request("GET /screenshot?scope=window HTTP/1.1", ""),
        &state,
    );

    assert_eq!(response.headers, [("X-Bones-Capture-Scope", "canvas")]);
}

#[test]
fn an_unknown_scope_is_a_bad_request() {
    let mut state = state();
    state.bus = Some(bones_kernel_infra::bus::Bus::new());
    let response = route(&request("GET /screenshot?scope=page HTTP/1.1", ""), &state);
    assert_eq!(response.status, "400 Bad Request");
}
