use std::sync::{Arc, Mutex};

use pubsub_bus::{BusEvent, Subscriber};

/// Every message on the bus (messaging.md). TODO: `correlation` stays
/// unused — synchronous send's (ADR-010) reply is the call's return value,
/// not a separate correlated message; kept for a future async pattern that
/// needs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub topic: String,
    pub sender: String,
    pub correlation: Option<u64>,
    pub payload: Vec<u8>,
}

/// Delivered-envelope callback; test doubles and real dispatch targets
/// (host, a module) both just implement this.
pub trait Handler: Send + Sync {
    fn handle(&mut self, envelope: &Envelope);
}

impl<F: FnMut(&Envelope) + Send + Sync> Handler for F {
    fn handle(&mut self, envelope: &Envelope) {
        self(envelope);
    }
}

/// Answers a direct `send` (ADR-010, messaging.md) with a reply, unlike
/// `Handler` which never returns anything back to the sender.
pub trait Respond: Send + Sync {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    UnknownEndpoint,
    /// TODO: dispatch is single-threaded today, so a target is never
    /// actually busy long enough to need waiting out — this variant exists
    /// for the contract (messaging.md) but nothing currently returns it.
    /// Meaningful once dispatch can run targets concurrently.
    Timeout,
    Cycle,
}

struct RegistryInner {
    targets: std::collections::HashMap<String, Arc<dyn Respond>>,
    /// Endpoints currently inside a synchronous call chain, outermost
    /// first — lets a request see its own name further up the chain and
    /// fail immediately instead of deadlocking (ADR-010).
    call_chain: Vec<String>,
}

/// Directory of endpoints reachable by name for direct `send` (ADR-010) —
/// separate from `Bus`'s pub/sub registration, which is by topic, not name.
/// Cheap to clone; every clone shares the same directory.
#[derive(Clone)]
pub struct Registry {
    inner: Arc<Mutex<RegistryInner>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryInner {
                targets: std::collections::HashMap::new(),
                call_chain: Vec::new(),
            })),
        }
    }

    pub fn insert(&self, name: impl Into<String>, target: Arc<dyn Respond>) {
        self.inner.lock().unwrap().targets.insert(name.into(), target);
    }

    pub fn remove(&self, name: &str) {
        self.inner.lock().unwrap().targets.remove(name);
    }

    /// Synchronously calls `to` on behalf of `from`, returning its reply
    /// (empty if `to` chose not to reply). Fails immediately — never waits —
    /// if `to` is already somewhere in the current call chain, `from`
    /// included (a cycle, self-sends counting as the shortest one, would
    /// otherwise deadlock both sides).
    pub fn call(&self, from: &str, to: &str, payload: &[u8]) -> Result<Vec<u8>, SendError> {
        let (target, pushed_root) = {
            let mut inner = self.inner.lock().unwrap();
            // `from` is already "running" even on the first call in a chain
            // (that's what's calling `send` right now) — pushing it before
            // the cycle check is what makes a self-send (from == to) count
            // as a cycle instead of slipping through.
            let pushed_root = inner.call_chain.is_empty();
            if pushed_root {
                inner.call_chain.push(from.to_string());
            }
            if inner.call_chain.iter().any(|name| name == to) {
                if pushed_root {
                    inner.call_chain.pop();
                }
                return Err(SendError::Cycle);
            }
            let Some(target) = inner.targets.get(to).cloned() else {
                if pushed_root {
                    inner.call_chain.pop();
                }
                return Err(SendError::UnknownEndpoint);
            };
            inner.call_chain.push(to.to_string());
            (target, pushed_root)
        };

        let reply = target.respond(from, payload);

        let mut inner = self.inner.lock().unwrap();
        inner.call_chain.pop(); // `to`
        if pushed_root {
            inner.call_chain.pop(); // `from`
        }

        Ok(reply.unwrap_or_default())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

fn topic_matches(pattern: &str, topic: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => topic.starts_with(prefix),
        None => pattern == topic,
    }
}

/// pubsub-bus's TopicId/source_id go unused (`()`/`0`) — routing happens
/// here, on `Envelope.topic` (ADR-013).
struct Adapter {
    patterns: Vec<String>,
    handler: Box<dyn Handler>,
}

impl Subscriber<Envelope, ()> for Adapter {
    fn is_subscribed_to(&self, _topic_id: &()) -> bool {
        true
    }

    fn on_event(&mut self, event: &BusEvent<Envelope, ()>) {
        let envelope = event.get_content();
        if self.patterns.iter().any(|p| topic_matches(p, &envelope.topic)) {
            self.handler.handle(envelope);
        }
    }
}

/// One persistent adapter per endpoint, for the engine's lifetime (ADR-013);
/// subscribe/release just mutate its pattern set.
#[derive(Clone)]
pub struct Endpoint {
    name: String,
    adapter: Arc<Mutex<Adapter>>,
}

impl Endpoint {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subscribe(&self, pattern: impl Into<String>) {
        self.adapter.lock().unwrap().patterns.push(pattern.into());
    }

    pub fn release(&self, pattern: &str) {
        self.adapter.lock().unwrap().patterns.retain(|p| p != pattern);
    }

    pub fn release_all(&self) {
        self.adapter.lock().unwrap().patterns.clear();
    }
}

/// Cheap to clone: an extension's `publish` import needs to hold a handle
/// to the same bus it's registered on. `pubsub-bus`'s `EventBus` is already
/// `Arc`-backed internally but doesn't derive `Clone` itself (vendored —
/// ADR-013 wraps it rather than modifying it), so the `Arc` here is bones'
/// own.
#[derive(Clone)]
pub struct Bus {
    inner: Arc<pubsub_bus::EventBus<Envelope, ()>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(pubsub_bus::EventBus::new()),
        }
    }

    pub fn register(&self, name: impl Into<String>, handler: impl Handler + 'static) -> Endpoint {
        let name = name.into();
        let adapter = Arc::new(Mutex::new(Adapter {
            patterns: Vec::new(),
            handler: Box::new(handler),
        }));
        self.inner.add_subscriber_shared(adapter.clone());
        Endpoint { name, adapter }
    }

    /// Fully removes an endpoint (unlike `Endpoint::release_all`, which
    /// only stops it matching — this drops the pubsub-bus registration
    /// itself). Idempotent; safe to call more than once.
    pub fn unregister(&self, endpoint: &Endpoint) {
        endpoint.release_all();
        let erased: Arc<Mutex<dyn Subscriber<Envelope, ()>>> = endpoint.adapter.clone();
        self.inner.remove_subscriber_shared(&erased);
    }

    /// Enqueues only — safe to call from inside a Handler; delivery waits
    /// for `dispatch()` (ADR-015).
    pub fn publish(&self, envelope: Envelope) {
        self.inner.enqueue(envelope, None, 0);
    }

    /// Delivers everything enqueued since the last call, in order (ADR-009).
    pub fn dispatch(&self) {
        self.inner.dispatch();
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(received.lock().unwrap().len(), 1, "unregistered endpoint must not be delivered to");
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

        assert!(received.lock().unwrap().is_empty(), "publish alone must not deliver");
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
        assert_eq!(registry.call("caller", "nobody", b"hi"), Err(SendError::UnknownEndpoint));
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

        assert_eq!(registry.call("caller", "echo", b"hi"), Err(SendError::UnknownEndpoint));
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

        assert_eq!(result, Ok(Vec::new()), "b's call back to a fails, but a's own call to b still completes");
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
}
