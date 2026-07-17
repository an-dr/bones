//! `core/lifecycle` (messaging.md, design/extensions.md): the topic every
//! extension state transition is published on, so tooling and other
//! extensions can observe loads, faults, and reloads.

use bones_messages::lifecycle::LifecycleEvent;
use bones_messages::{EncodeMessage, Message};
use bus::{Bus, Envelope};

pub use bones_messages::lifecycle::Event;

/// Publishes `name`'s transition on `core/lifecycle`, `sender` stamped as
/// the publishing component (e.g. `"engine"`), not `name` itself — the
/// event is about `name`, not from it.
pub fn publish(bus: &Bus, sender: &str, name: &str, event: Event) {
    let message = LifecycleEvent {
        event,
        extension: name,
    };
    bus.publish(Envelope {
        topic: LifecycleEvent::TOPIC.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: message.encode(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bones_messages::DecodeMessage;
    use std::sync::{Arc, Mutex};

    #[test]
    fn parse_recovers_what_publish_encoded() {
        let bus = Bus::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let ep = bus.register("watcher", move |e: &Envelope| {
            sink.lock().unwrap().push(e.clone())
        });
        ep.subscribe(LifecycleEvent::TOPIC);

        publish(&bus, "engine", "level", Event::Reloaded);
        bus.dispatch();

        let got = received.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].sender, "engine");
        assert_eq!(
            LifecycleEvent::decode(&got[0].payload),
            Ok(LifecycleEvent {
                event: Event::Reloaded,
                extension: "level"
            })
        );
    }
}
