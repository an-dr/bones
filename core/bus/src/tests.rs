use std::sync::{Arc, Mutex};

use crate::{
    BudgetLimits, Bus, DropCounters, EndpointBudget, Envelope, Handler, Registry, Respond,
    SendError,
};

fn envelope(topic: &str, sender: &str) -> Envelope {
    Envelope {
        topic: topic.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: Vec::new(),
    }
}

fn recording_handler() -> (impl Handler, Arc<Mutex<Vec<Envelope>>>) {
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    let handler = move |e: &Envelope| sink.lock().unwrap().push(e.clone());
    (handler, received)
}

#[test]
fn a_clone_publishes_into_the_same_bus() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");

    let clone = bus.clone();
    clone.publish(envelope("core/tick", "runner"));
    bus.dispatch();

    assert_eq!(received.lock().unwrap().len(), 1);
}

#[test]
fn exact_topic_is_delivered() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");

    bus.publish(envelope("core/tick", "runner"));
    bus.dispatch();

    assert_eq!(received.lock().unwrap().len(), 1);
}

#[test]
fn non_matching_topic_is_not_delivered() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");

    bus.publish(envelope("input/key-down", "platform"));
    bus.dispatch();

    assert!(received.lock().unwrap().is_empty());
}

#[test]
fn prefix_wildcard_matches() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("input/*");

    bus.publish(envelope("input/key-down", "platform"));
    bus.publish(envelope("input/mouse-move", "platform"));
    bus.publish(envelope("window/resize", "platform"));
    bus.dispatch();

    assert_eq!(received.lock().unwrap().len(), 2);
}

#[test]
fn release_stops_delivery() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");
    ep.release("core/tick");

    bus.publish(envelope("core/tick", "runner"));
    bus.dispatch();

    assert!(received.lock().unwrap().is_empty());
}

#[test]
fn release_all_clears_every_pattern() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");
    ep.subscribe("input/*");
    ep.release_all();

    bus.publish(envelope("core/tick", "runner"));
    bus.publish(envelope("input/key-down", "platform"));
    bus.dispatch();

    assert!(received.lock().unwrap().is_empty());
}

#[test]
fn only_matching_endpoints_receive_a_multicast_publish() {
    let bus = Bus::new();
    let (handler_a, received_a) = recording_handler();
    let (handler_b, received_b) = recording_handler();
    let ep_a = bus.register("ui", handler_a);
    let ep_b = bus.register("web", handler_b);
    ep_a.subscribe("ui/*");
    ep_b.subscribe("web/*");

    bus.publish(envelope("ui/clicked", "ui"));
    bus.dispatch();

    assert_eq!(received_a.lock().unwrap().len(), 1);
    assert!(received_b.lock().unwrap().is_empty());
}

#[test]
fn envelope_fields_survive_delivery() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("game/*");

    bus.publish(Envelope {
        topic: "game/spawn".to_string(),
        sender: "core".to_string(),
        correlation: Some(7),
        payload: vec![1, 2, 3],
    });
    bus.dispatch();

    let got = received.lock().unwrap();
    let e = &got[0];
    assert_eq!(e.sender, "core");
    assert_eq!(e.correlation, Some(7));
    assert_eq!(e.payload, vec![1, 2, 3]);
}

#[test]
fn endpoint_name_is_retained() {
    let bus = Bus::new();
    let ep = bus.register("level", |_: &Envelope| {});
    assert_eq!(ep.name(), "level");
}

#[test]
fn unregistered_endpoint_stops_receiving_events() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");

    bus.publish(envelope("core/tick", "runner"));
    bus.dispatch();
    assert_eq!(received.lock().unwrap().len(), 1);

    bus.unregister(&ep);
    bus.publish(envelope("core/tick", "runner"));
    bus.dispatch();
    assert_eq!(
        received.lock().unwrap().len(),
        1,
        "unregistered endpoint must not be delivered to"
    );
}

#[test]
fn unregister_is_idempotent() {
    let bus = Bus::new();
    let ep = bus.register("level", |_: &Envelope| {});

    bus.unregister(&ep);
    bus.unregister(&ep); // must not panic
}

#[test]
fn publish_without_dispatch_is_not_delivered() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("core/tick");

    bus.publish(envelope("core/tick", "runner"));

    assert!(
        received.lock().unwrap().is_empty(),
        "publish alone must not deliver"
    );
}

#[test]
fn a_single_dispatch_delivers_a_batch_in_fifo_order() {
    let bus = Bus::new();
    let (handler, received) = recording_handler();
    let ep = bus.register("level", handler);
    ep.subscribe("game/*");

    bus.publish(envelope("game/a", "core"));
    bus.publish(envelope("game/b", "core"));
    bus.publish(envelope("game/c", "core"));
    bus.dispatch();

    let got = received.lock().unwrap();
    let topics: Vec<&str> = got.iter().map(|e| e.topic.as_str()).collect();
    assert_eq!(topics, vec!["game/a", "game/b", "game/c"]);
}

#[test]
fn reactive_publish_from_a_handler_waits_for_the_next_dispatch() {
    // Regression test: without deferred dispatch this deadlocks
    // (ADR-015). Must complete without hanging.
    let bus = Arc::new(Bus::new());
    let (handler_b, received_b) = recording_handler();
    let ep_b = bus.register("b", handler_b);
    ep_b.subscribe("chain/b");

    let reactive_bus = bus.clone();
    let ep_a = bus.register("a", move |e: &Envelope| {
        if e.topic == "chain/a" {
            reactive_bus.publish(envelope("chain/b", "a"));
        }
    });
    ep_a.subscribe("chain/a");

    bus.publish(envelope("chain/a", "runner"));
    bus.dispatch();
    assert!(
        received_b.lock().unwrap().is_empty(),
        "reactively-enqueued envelope must not be delivered within the same dispatch"
    );

    bus.dispatch();
    assert_eq!(received_b.lock().unwrap().len(), 1);
}

#[test]
fn a_bounded_endpoint_drops_matching_deliveries_over_its_allowance() {
    let bus = Bus::new();
    let budget = EndpointBudget::new(BudgetLimits {
        max_inbound: 2,
        max_publishes: 1,
    });
    let (handler, received) = recording_handler();
    let endpoint = bus.register_with_budget("level", handler, budget.clone());
    endpoint.subscribe("game/*");

    bus.publish(envelope("game/a", "core"));
    bus.publish(envelope("input/key-down", "platform"));
    bus.publish(envelope("game/b", "core"));
    bus.publish(envelope("game/c", "core"));
    bus.dispatch();

    let topics: Vec<_> = received
        .lock()
        .unwrap()
        .iter()
        .map(|envelope| envelope.topic.clone())
        .collect();
    assert_eq!(topics, vec!["game/a", "game/b"]);
    assert_eq!(
        budget.get_drop_counters(),
        DropCounters {
            inbound: 1,
            publishes: 0,
        }
    );
    assert!(budget.has_exceeded());
}

#[test]
fn begin_frame_restores_allowances_without_clearing_violation_history() {
    let budget = EndpointBudget::new(BudgetLimits {
        max_inbound: 1,
        max_publishes: 1,
    });

    assert!(budget.accept_publish());
    assert!(!budget.accept_publish());
    budget.begin_frame();
    assert!(budget.accept_publish());

    assert!(budget.has_exceeded());
    assert_eq!(budget.get_drop_counters().publishes, 1);
}

#[test]
fn budget_clones_share_allowances_and_counters() {
    let budget = EndpointBudget::new(BudgetLimits {
        max_inbound: 1,
        max_publishes: 0,
    });
    let clone = budget.clone();

    assert!(budget.accept_inbound());
    assert!(!clone.accept_inbound());
    assert!(!clone.accept_publish());

    assert_eq!(
        budget.get_drop_counters(),
        DropCounters {
            inbound: 1,
            publishes: 1,
        }
    );
}

struct Echo;
impl Respond for Echo {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        let mut reply = sender.as_bytes().to_vec();
        reply.extend_from_slice(payload);
        Some(reply)
    }
}

struct Silent;
impl Respond for Silent {
    fn respond(&self, _sender: &str, _payload: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

struct Forwarding {
    registry: Registry,
    target: String,
}
impl Respond for Forwarding {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.registry.call(sender, &self.target, payload).ok()
    }
}

#[test]
fn call_reaches_the_named_target_and_returns_its_reply() {
    let registry = Registry::new();
    registry.insert("echo", Arc::new(Echo));

    let reply = registry.call("caller", "echo", b"hi").unwrap();

    assert_eq!(reply, b"callerhi");
}

#[test]
fn call_to_an_unknown_endpoint_errors() {
    let registry = Registry::new();
    assert_eq!(
        registry.call("caller", "nobody", b"hi"),
        Err(SendError::UnknownEndpoint)
    );
}

#[test]
fn a_target_choosing_not_to_reply_gets_an_empty_reply() {
    let registry = Registry::new();
    registry.insert("silent", Arc::new(Silent));

    assert_eq!(registry.call("caller", "silent", b"hi"), Ok(Vec::new()));
}

#[test]
fn removed_endpoint_is_unreachable() {
    let registry = Registry::new();
    registry.insert("echo", Arc::new(Echo));
    registry.remove("echo");

    assert_eq!(
        registry.call("caller", "echo", b"hi"),
        Err(SendError::UnknownEndpoint)
    );
}

#[test]
fn a_self_send_is_a_cycle_of_one_instead_of_deadlocking() {
    let registry = Registry::new();
    registry.insert("echo", Arc::new(Echo));

    assert_eq!(registry.call("echo", "echo", b"hi"), Err(SendError::Cycle));
}

#[test]
fn a_direct_cycle_fails_immediately_instead_of_deadlocking() {
    let registry = Registry::new();
    // "a" calls "b", whose handler calls back into "a" — must not hang.
    registry.insert(
        "b",
        Arc::new(Forwarding {
            registry: registry.clone(),
            target: "a".to_string(),
        }),
    );
    registry.insert(
        "a",
        Arc::new(Forwarding {
            registry: registry.clone(),
            target: "b".to_string(),
        }),
    );

    let result = registry.call("test", "a", b"go");

    assert_eq!(
        result,
        Ok(Vec::new()),
        "b's call back to a fails, but a's own call to b still completes"
    );
}

#[test]
fn call_chain_is_clear_again_after_a_cycle_is_rejected() {
    let registry = Registry::new();
    registry.insert(
        "b",
        Arc::new(Forwarding {
            registry: registry.clone(),
            target: "a".to_string(),
        }),
    );
    registry.insert(
        "a",
        Arc::new(Forwarding {
            registry: registry.clone(),
            target: "b".to_string(),
        }),
    );
    registry.call("test", "a", b"go").unwrap();

    // A fresh, independent call must not be mistaken for still being
    // inside the previous (already-finished) call chain.
    registry.insert("echo", Arc::new(Echo));
    let reply = registry.call("test", "echo", b"x");

    assert_eq!(reply, Ok(b"testx".to_vec()));
}
