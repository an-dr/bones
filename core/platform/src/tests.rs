use super::*;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::sync::{Mutex, OnceLock};

// SDL can't handle two concurrent `Platform::new` calls opening real
// windows (observed as an assertion failure deep in SDL's pen-input
// init, then a hang) — cargo runs #[test]s in parallel by default, so
// every test that opens a real window takes this lock to never run
// concurrently with another.
fn sdl_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn key_down_becomes_an_input_key_down_envelope() {
    let event = Event::KeyDown {
        timestamp: 0,
        window_id: 0,
        keycode: Some(Keycode::A),
        scancode: None,
        keymod: sdl3::keyboard::Mod::empty(),
        repeat: false,
        which: 0,
        raw: 0,
    };

    let envelope = translate_event(&event, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/key-down");
    assert_eq!(envelope.sender, "platform");
    assert_eq!(envelope.payload, b"A");
}

#[test]
fn key_up_becomes_an_input_key_up_envelope() {
    let event = Event::KeyUp {
        timestamp: 0,
        window_id: 0,
        keycode: Some(Keycode::Space),
        scancode: None,
        keymod: sdl3::keyboard::Mod::empty(),
        repeat: false,
        which: 0,
        raw: 0,
    };

    let envelope = translate_event(&event, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/key-up");
    assert_eq!(envelope.payload, b"Space");
}

#[test]
fn unrelated_events_translate_to_nothing() {
    let event = Event::Quit { timestamp: 0 };
    assert!(translate_event(&event, "platform").is_none());
}

#[test]
fn mouse_button_down_becomes_an_input_mouse_down_envelope() {
    let event = Event::MouseButtonDown {
        timestamp: 0,
        window_id: 0,
        which: 0,
        mouse_btn: sdl3::mouse::MouseButton::Right,
        clicks: 1,
        x: 12.0,
        y: 34.0,
    };
    let envelope = translate_event(&event, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/mouse-down");
    assert_eq!(
        envelope.payload,
        bones_messages::input::MouseDown { button: 3, x: 12.0, y: 34.0 }.encode()
    );
}

#[test]
fn mouse_wheel_normalizes_flipped_direction() {
    let normal = Event::MouseWheel {
        timestamp: 0,
        window_id: 0,
        which: 0,
        x: 1.0,
        y: 2.0,
        direction: sdl3::mouse::MouseWheelDirection::Normal,
        mouse_x: 0.0,
        mouse_y: 0.0,
        integer_x: 1,
        integer_y: 2,
    };
    let envelope = translate_event(&normal, "platform").expect("should translate");
    assert_eq!(
        envelope.payload,
        bones_messages::input::MouseWheel { x: 1.0, y: 2.0 }.encode()
    );

    let flipped = Event::MouseWheel {
        timestamp: 0,
        window_id: 0,
        which: 0,
        x: 1.0,
        y: 2.0,
        direction: sdl3::mouse::MouseWheelDirection::Flipped,
        mouse_x: 0.0,
        mouse_y: 0.0,
        integer_x: 1,
        integer_y: 2,
    };
    let envelope = translate_event(&flipped, "platform").expect("should translate");
    assert_eq!(
        envelope.payload,
        bones_messages::input::MouseWheel { x: -1.0, y: -2.0 }.encode(),
        "flipped direction should negate x/y so consumers see one consistent sign convention"
    );
}

#[test]
fn gamepad_axis_motion_becomes_a_normalized_input_gamepad_axis_envelope() {
    let event = Event::ControllerAxisMotion {
        timestamp: 0,
        which: 3,
        axis: sdl3::gamepad::Axis::LeftX,
        value: i16::MAX,
    };
    let envelope = translate_event(&event, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/gamepad-axis");
    assert_eq!(
        envelope.payload,
        bones_messages::input::GamepadAxis { id: 3, axis: "LeftX", value: 1.0 }.encode()
    );

    let negative = Event::ControllerAxisMotion {
        timestamp: 0,
        which: 3,
        axis: sdl3::gamepad::Axis::LeftX,
        value: i16::MIN,
    };
    let envelope = translate_event(&negative, "platform").expect("should translate");
    assert_eq!(
        envelope.payload,
        bones_messages::input::GamepadAxis { id: 3, axis: "LeftX", value: -1.0 }.encode(),
        "i16::MIN is more negative than -i16::MAX; must clamp to -1.0, not overshoot"
    );
}

#[test]
fn gamepad_button_events_become_input_gamepad_button_envelopes() {
    let down = Event::ControllerButtonDown {
        timestamp: 0,
        which: 3,
        button: sdl3::gamepad::Button::South,
    };
    let envelope = translate_event(&down, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/gamepad-button-down");
    assert_eq!(
        envelope.payload,
        bones_messages::input::GamepadButtonDown { id: 3, button: "South" }.encode()
    );

    let up = Event::ControllerButtonUp {
        timestamp: 0,
        which: 3,
        button: sdl3::gamepad::Button::South,
    };
    let envelope = translate_event(&up, "platform").expect("should translate");
    assert_eq!(envelope.topic, "input/gamepad-button-up");
}

#[test]
fn an_injected_quit_event_sets_quit_requested_instead_of_publishing() {
    let _guard = sdl_test_lock().lock().unwrap();
    let mut platform = Platform::new("test", 64, 64).expect("needs a real display");
    platform
        .inject_event(Event::Quit { timestamp: 0 })
        .expect("inject should succeed");

    let bus = Bus::new();
    assert!(!platform.quit_requested(), "must not be set before polling");

    platform.poll_events(&bus, "platform");

    assert!(
        platform.quit_requested(),
        "expected quit_requested after polling a Quit event"
    );
}

#[test]
fn an_injected_key_event_round_trips_through_a_real_window_onto_the_bus() {
    let _guard = sdl_test_lock().lock().unwrap();
    let mut platform = Platform::new("test", 64, 64).expect("needs a real display");
    platform
        .inject_event(Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::A),
            scancode: Some(sdl3::keyboard::Scancode::A),
            keymod: sdl3::keyboard::Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        })
        .expect("inject should succeed");

    let bus = Bus::new();
    let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = received.clone();
    let ep = bus.register("test", move |e: &Envelope| {
        sink.lock().unwrap().push(e.clone())
    });
    ep.subscribe(KeyDown::TOPIC);

    platform.poll_events(&bus, "platform");
    bus.dispatch();

    let got = received.lock().unwrap();
    assert!(
        got.iter()
            .any(|e| e.topic == "input/key-down" && e.payload == b"A"),
        "expected an injected key-down to reach the bus, got {got:?}"
    );
}

#[test]
fn a_key_event_consumed_by_a_higher_layer_never_reaches_input() {
    let _guard = sdl_test_lock().lock().unwrap();
    let mut platform = Platform::new("test", 64, 64).expect("needs a real display");
    platform
        .inject_event(Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::A),
            scancode: Some(sdl3::keyboard::Scancode::A),
            keymod: sdl3::keyboard::Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        })
        .expect("inject should succeed");

    let bus = Bus::new();
    let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = received.clone();
    let ep = bus.register("test", move |e: &Envelope| {
        sink.lock().unwrap().push(e.clone())
    });
    ep.subscribe(KeyDown::TOPIC);

    platform.poll_events_with(&bus, "platform", |_event| true);
    bus.dispatch();

    assert!(
        received.lock().unwrap().is_empty(),
        "a consumed event must not be translated onto input/*"
    );
}

#[test]
fn an_injected_quit_event_bypasses_the_consumer_hook() {
    let _guard = sdl_test_lock().lock().unwrap();
    let mut platform = Platform::new("test", 64, 64).expect("needs a real display");
    platform
        .inject_event(Event::Quit { timestamp: 0 })
        .expect("inject should succeed");

    let bus = Bus::new();
    platform.poll_events_with(&bus, "platform", |_event| true);

    assert!(
        platform.quit_requested(),
        "Quit must set quit_requested even when the consumer hook claims everything"
    );
}
