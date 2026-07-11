//! Platform layer (design/platform.md): the only component touching the OS.
//! This rung: one SDL window, keyboard events onto `input/*` (ADR-008's
//! web/egui layers don't exist yet, so every event reaches `input/*`
//! directly). Window/tray/mouse/controller/timing are later work.

use bus::{Bus, Envelope};

pub struct Platform {
    sdl: sdl3::Sdl,
    _window: sdl3::video::Window,
    events: sdl3::EventPump,
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
        let events = sdl.event_pump().map_err(|e| e.to_string())?;

        Ok(Self {
            sdl,
            _window: window,
            events,
        })
    }

    /// Publishes an `input/*` envelope for every pending keyboard event.
    /// Only enqueues (`Bus::publish`) — the caller decides when to dispatch.
    pub fn poll_events(&mut self, bus: &Bus, sender: &str) {
        for event in self.events.poll_iter() {
            if let Some(envelope) = translate_event(&event, sender) {
                bus.publish(envelope);
            }
        }
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
    let (topic, key) = match event {
        sdl3::event::Event::KeyDown {
            keycode: Some(key), ..
        } => ("input/key-down", key),
        sdl3::event::Event::KeyUp {
            keycode: Some(key), ..
        } => ("input/key-up", key),
        _ => return None,
    };
    Some(Envelope {
        topic: topic.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: key.to_string().into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl3::event::Event;
    use sdl3::keyboard::Keycode;

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
    fn an_injected_key_event_round_trips_through_a_real_window_onto_the_bus() {
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
        let ep = bus.register("test", move |e: &Envelope| sink.lock().unwrap().push(e.clone()));
        ep.subscribe("input/key-down");

        platform.poll_events(&bus, "platform");
        bus.dispatch();

        let got = received.lock().unwrap();
        assert!(
            got.iter().any(|e| e.topic == "input/key-down" && e.payload == b"A"),
            "expected an injected key-down to reach the bus, got {got:?}"
        );
    }
}
