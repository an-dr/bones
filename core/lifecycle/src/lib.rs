//! `core/lifecycle` (messaging.md, design/extensions.md): the topic every
//! extension state transition is published on, so tooling and other
//! extensions can observe loads, faults, and reloads.

use bus::{Bus, Envelope};
use buffer_rw::{Reader, Writer};

pub const TOPIC: &str = "core/lifecycle";

/// A transition in the state diagram (design/extensions.md). Loaded/Stopped
/// bookend a normal lifetime; Faulted/Reloading/Reloaded are ADR-007/hot
/// reload's doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Loaded,
    Faulted,
    Reloading,
    Reloaded,
    Stopped,
}

impl Event {
    fn tag(self) -> u8 {
        match self {
            Event::Loaded => 0,
            Event::Faulted => 1,
            Event::Reloading => 2,
            Event::Reloaded => 3,
            Event::Stopped => 4,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, String> {
        match tag {
            0 => Ok(Event::Loaded),
            1 => Ok(Event::Faulted),
            2 => Ok(Event::Reloading),
            3 => Ok(Event::Reloaded),
            4 => Ok(Event::Stopped),
            other => Err(format!("unknown lifecycle event tag {other}")),
        }
    }
}

/// Publishes `name`'s transition on `core/lifecycle`, `sender` stamped as
/// the publishing component (e.g. `"engine"`), not `name` itself — the
/// event is about `name`, not from it.
pub fn publish(bus: &Bus, sender: &str, name: &str, event: Event) {
    let payload = Writer::new().u8(event.tag()).bytes(name.as_bytes()).finish();
    bus.publish(Envelope {
        topic: TOPIC.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload,
    });
}

/// Recovers the `(event, extension name)` pair `publish` encoded.
pub fn parse(payload: &[u8]) -> Result<(Event, String), String> {
    let mut r = Reader::new(payload);
    let tag = r.read_u8().map_err(|e| e.to_string())?;
    let event = Event::from_tag(tag)?;
    let name = String::from_utf8(r.read_rest().to_vec()).map_err(|e| e.to_string())?;
    Ok((event, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn parse_recovers_what_publish_encoded() {
        let bus = Bus::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let ep = bus.register("watcher", move |e: &Envelope| sink.lock().unwrap().push(e.clone()));
        ep.subscribe(TOPIC);

        publish(&bus, "engine", "level", Event::Reloaded);
        bus.dispatch();

        let got = received.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].sender, "engine");
        assert_eq!(parse(&got[0].payload), Ok((Event::Reloaded, "level".to_string())));
    }

    #[test]
    fn every_event_round_trips() {
        for event in [Event::Loaded, Event::Faulted, Event::Reloading, Event::Reloaded, Event::Stopped] {
            let payload = Writer::new().u8(event.tag()).bytes(b"ext").finish();
            assert_eq!(parse(&payload), Ok((event, "ext".to_string())));
        }
    }

    #[test]
    fn an_unknown_tag_is_an_error() {
        let payload = Writer::new().u8(255).bytes(b"ext").finish();
        assert!(parse(&payload).is_err());
    }
}
