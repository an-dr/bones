//! Platform layer (design/platform.md): the only component touching the OS.
//! Opens one SDL window, publishes keyboard events onto `input/*`. TODO:
//! tray, mouse, controller, and timing sources aren't published onto
//! `input/*` yet — mouse/text-input events are captured (`poll_events_with`)
//! but only ever reach the ui module's consumption hook, never the bus.

use bones_messages::input::{KeyDown, KeyUp};
use bones_messages::{EncodeMessage, Message};
use bus::{Bus, Envelope};

pub struct Platform {
    sdl: sdl3::Sdl,
    // `Option` so the window can be handed to a renderer later (`take_window`)
    // — event polling only needs the `Sdl` context, not this value itself.
    window: Option<sdl3::video::Window>,
    events: sdl3::EventPump,
    quit_requested: bool,
}

impl Platform {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let video = sdl.video().map_err(|e| e.to_string())?;
        let window = video
            .window(title, width, height)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        // Started unconditionally, before the window can be handed away via
        // `take_window` — text input has no per-window "focused a text
        // field" signal yet, so this just keeps `Event::TextInput` flowing
        // for whichever layer (ui) wants to consume it. TODO: gate on focus
        // once one exists, for IME/mobile-keyboard hygiene.
        video.text_input().start(&window);
        let events = sdl.event_pump().map_err(|e| e.to_string())?;

        Ok(Self {
            sdl,
            window: Some(window),
            events,
            quit_requested: false,
        })
    }

    /// Hands the window over (e.g. to a renderer needing it for a `Canvas`).
    /// `None` if already taken. Keyboard input keeps working either way —
    /// `poll_events` only needs the `Sdl` context, never this value.
    pub fn take_window(&mut self) -> Option<sdl3::video::Window> {
        self.window.take()
    }

    /// Publishes an `input/*` envelope for every pending keyboard event, and
    /// records a window-close request for `quit_requested` to report.
    /// Only enqueues (`Bus::publish`) — the caller decides when to dispatch.
    pub fn poll_events(&mut self, bus: &Bus, sender: &str) {
        self.poll_events_with(bus, sender, |_| false);
    }

    /// Same as `poll_events`, but offers every non-`Quit` raw event to
    /// `consumed` first (ADR-008: top layer consumes) — a higher layer
    /// (e.g. the ui module's egui context) that claims an event by
    /// returning `true` stops it from ever reaching `input/*`. `Quit` is a
    /// session-lifecycle signal, not layered input, so it always bypasses
    /// `consumed`.
    pub fn poll_events_with(
        &mut self,
        bus: &Bus,
        sender: &str,
        mut consumed: impl FnMut(&sdl3::event::Event) -> bool,
    ) {
        for event in self.events.poll_iter() {
            if matches!(event, sdl3::event::Event::Quit { .. }) {
                self.quit_requested = true;
                continue;
            }
            if consumed(&event) {
                continue;
            }
            if let Some(envelope) = translate_event(&event, sender) {
                bus.publish(envelope);
            }
        }
    }

    /// Whether the OS asked to close the window (e.g. the close button)
    /// since this `Platform` was created. Sticky — once true, stays true;
    /// the caller is expected to exit rather than keep polling.
    /// TODO: no `window/*` close-request event or `shutdown()` call yet
    /// (design/platform.md's full shutdown sequence) — this only reports
    /// the signal, exiting cleanly is the caller's job.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// Injects a synthetic event into SDL's own queue, as if the OS had
    /// delivered it — for tests/tooling that simulate input without a real
    /// keypress. The next `poll_events` picks it up like any other event.
    /// For `KeyDown`/`KeyUp`, `scancode` must be `Some(..)` — SDL's own
    /// conversion needs it even though `translate_event` never reads it.
    pub fn inject_event(&self, event: sdl3::event::Event) -> Result<(), String> {
        self.sdl
            .event()
            .map_err(|e| e.to_string())?
            .push_event(event)
            .map_err(|e| e.to_string())
    }
}

fn translate_event(event: &sdl3::event::Event, sender: &str) -> Option<Envelope> {
    let (topic, payload) = match event {
        sdl3::event::Event::KeyDown {
            keycode: Some(key), ..
        } => {
            let key = key.to_string();
            (KeyDown::TOPIC, KeyDown { key: &key }.encode())
        }
        sdl3::event::Event::KeyUp {
            keycode: Some(key), ..
        } => {
            let key = key.to_string();
            (KeyUp::TOPIC, KeyUp { key: &key }.encode())
        }
        _ => return None,
    };
    Some(Envelope {
        topic: topic.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload,
    })
}

#[cfg(test)]
mod tests {
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
}
