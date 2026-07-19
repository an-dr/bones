use pubsub_bus::{BusEvent, Subscriber};

use crate::{Envelope, Handler};

fn topic_matches(pattern: &str, topic: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => topic.starts_with(prefix),
        None => pattern == topic,
    }
}

/// pubsub-bus's TopicId/source_id go unused (`()`/`0`) — routing happens
/// here, on `Envelope.topic` (ADR-013).
pub(crate) struct Adapter {
    pub(crate) patterns: Vec<String>,
    pub(crate) handler: Box<dyn Handler>,
}

impl Subscriber<Envelope, ()> for Adapter {
    fn is_subscribed_to(&self, _topic_id: &()) -> bool {
        true
    }

    fn on_event(&mut self, event: &BusEvent<Envelope, ()>) {
        let envelope = event.get_content();
        if self
            .patterns
            .iter()
            .any(|p| topic_matches(p, &envelope.topic))
        {
            self.handler.handle(envelope);
        }
    }
}
