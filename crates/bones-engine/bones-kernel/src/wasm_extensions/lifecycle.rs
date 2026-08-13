//! The topic every extension state transition is published on
//! (messaging.md, design/extensions.md), so tooling and other extensions
//! can observe loads, faults, and reloads.

use bones_messages::lifecycle::LifecycleEvent;
use bones_messages::{EncodeMessage, Message};
use crate::bus::{Bus, Envelope};

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
mod tests;
