use std::sync::{Arc, Mutex};
use pubsub_bus::Subscriber;

use crate::adapter::Adapter;
use crate::{Endpoint, Envelope, Handler};

/// Cheap to clone: an extension's `publish` import needs to hold a handle
/// to the same bus it's registered on. `pubsub_bus::EventBus` is `Clone`
/// (3.2.0) and already `Arc`-backed internally, so no wrapping `Arc` of
/// bones' own is needed here.
#[derive(Clone)]
pub struct Bus {
    inner: pubsub_bus::EventBus<Envelope, ()>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            inner: pubsub_bus::EventBus::new(),
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
