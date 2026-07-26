#[cfg(target_os = "windows")]
use bones_messages::web::PanelSource;
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

use super::*;

#[cfg(not(target_os = "windows"))]
#[test]
fn backend_captures_an_sdl_parent_window() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("bones web smoke", 160, 120).build().unwrap();

    let mut backend = WryBackend::new(&window).unwrap();

    assert!(backend.drain_events().is_empty());
}

#[cfg(target_os = "windows")]
#[test]
fn child_webview_opens_receives_script_and_closes() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let mut events = sdl.event_pump().unwrap();
    let window = video
        .window("bones webview smoke", 160, 120)
        .build()
        .unwrap();
    let mut backend = WryBackend::new(&window).unwrap();

    backend
        .open(
            "smoke",
            "panel",
            PanelSource::Html(
                "<!doctype html><title>bones smoke</title><script>\
                 window.addEventListener('bones-message', event => \
                   window.ipc.postMessage(event.detail));\
                 window.ipc.postMessage('ready');\
                 </script>",
            ),
        )
        .unwrap();
    assert_eq!(
        wait_for_message(&mut backend, &mut events),
        Some("ready".to_string())
    );
    backend
        .send_json("smoke", "panel", r#"{"ready":true}"#)
        .unwrap();
    assert_eq!(
        wait_for_message(&mut backend, &mut events),
        Some(r#"{"ready":true}"#.to_string())
    );
    backend.navigate("smoke", "panel", "about:blank").unwrap();
    backend.close("smoke", "panel").unwrap();
}

#[cfg(target_os = "windows")]
fn wait_for_message(backend: &mut WryBackend, events: &mut sdl3::EventPump) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for _ in events.poll_iter() {}
        if let Some(BackendEvent::PageMessage { json, .. }) =
            backend.drain_events().into_iter().next()
        {
            return Some(json);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}
