#[cfg(any(target_os = "windows", target_os = "linux"))]
use bones_messages::web::PanelSource;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::time::{Duration, Instant};

use super::*;

mod sdl_thread;

macro_rules! sdl_test {
    ($(#[$attr:meta])* fn $name:ident() $body:block) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            sdl_thread::run(move || $body)
        }
    };
}

sdl_test! {
#[cfg(not(target_os = "windows"))]
fn backend_captures_an_sdl_parent_window() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("bones web smoke", 160, 120).build().unwrap();

    let inherited_backend = std::env::var_os("GDK_BACKEND");
    let mut backend = WryBackend::new(&window).unwrap();
    assert_eq!(std::env::var_os("GDK_BACKEND"), inherited_backend);
    #[cfg(target_os = "linux")]
    assert!(std::thread::spawn(init_gtk).join().unwrap().is_err());

    assert!(backend.drain_events().is_empty());
}}

sdl_test! {
#[cfg(target_os = "windows")]
fn child_webview_opens_resizes_receives_script_and_closes() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let mut events = sdl.event_pump().unwrap();
    let mut window = video
        .window("bones webview smoke", 160, 120)
        .resizable()
        .build()
        .unwrap();
    let mut backend = WryBackend::new(&window).unwrap();

    backend
        .open(
            "smoke",
            "panel",
            PanelSource::Html(
                "<!doctype html><title>bones smoke</title><script>\
                 window.addEventListener('bones-message', event => {\
                   if (event.detail === 'report-size') {\
                     window.ipc.postMessage(`size:${window.innerWidth}x${window.innerHeight}`);\
                   } else if (event.detail === 'report-storage') {\
                     try {\
                       localStorage.setItem('bones-smoke', 'kept');\
                       window.ipc.postMessage(`storage:${localStorage.getItem('bones-smoke')}`);\
                     } catch (error) {\
                       window.ipc.postMessage('storage:blocked');\
                     }\
                   } else {\
                     window.ipc.postMessage(event.detail);\
                   }\
                 });\
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
    // Storage is what an opaque origin denies, so a page served with a real
    // origin has to be able to reach it.
    backend
        .send_json("smoke", "panel", "report-storage")
        .unwrap();
    assert_eq!(
        wait_for_message_matching(&mut backend, &mut events, |message| {
            message.starts_with("storage:")
        }),
        Some("storage:kept".to_string()),
        "an html panel should load at an origin that allows storage"
    );

    backend.send_json("smoke", "panel", "report-size").unwrap();
    let initial_size = wait_for_message_matching(&mut backend, &mut events, |message| {
        message.starts_with("size:")
    })
    .expect("page should report its initial viewport");

    window.set_size(320, 240).unwrap();
    wait_for_window_size(&window, &mut events, (320, 240));
    backend.update().unwrap();
    let expected_pixels = window.size_in_pixels();
    assert_eq!(backend.pixel_size, expected_pixels);
    assert_eq!(
        backend
            .panel_mut("smoke", "panel")
            .unwrap()
            .bounds()
            .unwrap()
            .size,
        PhysicalSize::new(expected_pixels.0 as i32, expected_pixels.1 as i32).into()
    );
    backend.send_json("smoke", "panel", "report-size").unwrap();
    let reported_size = wait_for_message_matching(&mut backend, &mut events, |message| {
        message.starts_with("size:")
    });
    assert!(
        reported_size.is_some_and(|size| size != initial_size),
        "page viewport should change from {initial_size}"
    );
    backend.navigate("smoke", "panel", "about:blank").unwrap();
    backend.close("smoke", "panel").unwrap();
}}

// A constructor-only test cannot detect a missing GLib pump: the page must
// actually execute and deliver IPC before this deadline expires.
sdl_test! {
#[cfg(target_os = "linux")]
fn child_webview_loads_and_exchanges_messages_on_linux() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let mut events = sdl.event_pump().unwrap();
    let window = video.window("bones Linux webview smoke", 160, 120).build().unwrap();
    let mut backend = WryBackend::new(&window).unwrap();
    backend.open("smoke", "panel", PanelSource::Html(
        "<!doctype html><title>bones Linux smoke</title><script>\
         window.addEventListener('bones-message', event => {\
           window.ipc.postMessage(event.detail);\
         });\
         window.ipc.postMessage('ready');\
         </script>",
    )).unwrap();

    assert_eq!(wait_for_message(&mut backend, &mut events), Some("ready".into()));
    backend.send_json("smoke", "panel", "round-trip").unwrap();
    assert_eq!(wait_for_message(&mut backend, &mut events), Some("round-trip".into()));
    backend.close("smoke", "panel").unwrap();
}}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn wait_for_message(backend: &mut WryBackend, events: &mut sdl3::EventPump) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        for _ in events.poll_iter() {}
        backend.update().unwrap();
        if let Some(BackendEvent::PageMessage { json, .. }) =
            backend.drain_events().into_iter().next()
        {
            return Some(json);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

#[cfg(target_os = "windows")]
fn wait_for_window_size(
    window: &sdl3::video::Window,
    events: &mut sdl3::EventPump,
    expected: (u32, u32),
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for _ in events.poll_iter() {}
        if window.size() == expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "SDL window did not reach {expected:?}; actual size is {:?}",
        window.size()
    );
}

#[cfg(target_os = "windows")]
fn wait_for_message_matching(
    backend: &mut WryBackend,
    events: &mut sdl3::EventPump,
    matches: impl Fn(&str) -> bool,
) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for _ in events.poll_iter() {}
        backend.update().unwrap();
        for event in backend.drain_events() {
            if let BackendEvent::PageMessage { json, .. } = event {
                if matches(&json) {
                    return Some(json);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}
