use std::sync::Arc;

use bones_kernel_infra::bus::{Endpoint, Envelope};
use bones_kernel_infra::logging::{Level, RecordingSink};
use bones_messages::gfx::Clear;
use bones_messages::{EncodeMessage, Message};

use super::*;

fn recording() -> (Logger, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    (Logger::new(sink.clone()), sink)
}

/// A bus where `topics` have a subscriber, and the endpoint holding them,
/// which must stay alive: the bus keeps only a weak reference to it.
fn bus_hearing(topics: &[&str]) -> (Bus, Endpoint) {
    let bus = Bus::new();
    let endpoint = bus.register("listener", |_: &Envelope| {});
    for topic in topics {
        endpoint.subscribe(*topic);
    }
    (bus, endpoint)
}

fn lines(sink: &RecordingSink) -> Vec<(Level, String)> {
    sink.records()
        .into_iter()
        .map(|(level, _, line)| (level, line))
        .collect()
}

fn clear() -> Vec<u8> {
    Clear {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }
    .encode()
}

#[test]
fn a_malformed_core_payload_is_reported_once_with_its_explanation() {
    let (logger, sink) = recording();
    let (bus, _endpoint) = bus_hearing(&[Clear::TOPIC]);
    let mut diagnostics = Diagnostics::new();
    for _ in 0..3 {
        diagnostics.check_publish(&logger, &bus, "pong", Clear::TOPIC, &[1, 2]);
    }
    let lines = lines(&sink);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].1.contains("gfx/clear"), "{lines:?}");
    assert!(lines[0].1.contains("wit/wire-format.md"), "{lines:?}");
}

#[test]
fn a_heard_well_formed_payload_is_not_reported() {
    let (logger, sink) = recording();
    let (bus, _endpoint) = bus_hearing(&[Clear::TOPIC, "pong/score"]);
    let mut diagnostics = Diagnostics::new();
    diagnostics.check_publish(&logger, &bus, "pong", Clear::TOPIC, &clear());
    diagnostics.check_publish(&logger, &bus, "pong", "pong/score", &[1, 2]);
    assert!(sink.records().is_empty());
}

#[test]
fn an_unheard_core_topic_warns_that_its_module_is_off() {
    let (logger, sink) = recording();
    let (bus, _endpoint) = bus_hearing(&[]);
    let mut diagnostics = Diagnostics::new();
    diagnostics.check_publish(&logger, &bus, "pong", Clear::TOPIC, &clear());
    let lines = lines(&sink);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].0, Level::Warn);
    assert!(lines[0].1.contains("not enabled"), "{lines:?}");
}

#[test]
fn a_misspelt_topic_warns_once_with_the_topic_it_meant() {
    let (logger, sink) = recording();
    let (bus, _endpoint) = bus_hearing(&[Clear::TOPIC]);
    let mut diagnostics = Diagnostics::new();
    for _ in 0..3 {
        diagnostics.check_publish(&logger, &bus, "pong", "gfx/clearr", &clear());
    }
    let lines = lines(&sink);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].0, Level::Warn);
    assert!(lines[0].1.contains("did you mean 'gfx/clear'"), "{lines:?}");
}

#[test]
fn an_unheard_application_topic_is_only_a_debug_line() {
    let (logger, sink) = recording();
    let (bus, _endpoint) = bus_hearing(&[]);
    let mut diagnostics = Diagnostics::new();
    diagnostics.check_publish(&logger, &bus, "pong", "pong/score", &[1]);
    let lines = lines(&sink);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].0, Level::Debug);
}

#[test]
fn a_misspelt_subscription_warns_once_with_the_topic_it_meant() {
    let (logger, sink) = recording();
    let mut diagnostics = Diagnostics::new();
    for _ in 0..2 {
        diagnostics.check_subscribe(&logger, "orbit", "input/key-dwn");
    }
    let lines = lines(&sink);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].1.contains("did you mean 'input/key-down'"),
        "{lines:?}"
    );
}

#[test]
fn core_application_and_prefix_subscriptions_are_quiet() {
    let (logger, sink) = recording();
    let mut diagnostics = Diagnostics::new();
    diagnostics.check_subscribe(&logger, "orbit", "input/key-down");
    diagnostics.check_subscribe(&logger, "orbit", "orbit/score");
    diagnostics.check_subscribe(&logger, "orbit", "input/*");
    assert!(sink.records().is_empty());
}
