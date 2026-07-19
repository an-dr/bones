use super::ServiceRegistry;

/// Passed to `Module::init`: lets a module request bus subscriptions and
/// reach the service registry, without handing it the `Bus`/builder
/// internals directly.
pub struct ModuleContext<'a> {
    subscriptions: Vec<String>,
    registry: &'a mut ServiceRegistry,
}

impl<'a> ModuleContext<'a> {
    pub fn new(registry: &'a mut ServiceRegistry) -> Self {
        Self {
            subscriptions: Vec::new(),
            registry,
        }
    }

    /// Requests a bus subscription, applied by the caller after this
    /// module registers (mirrors how a WASM extension's `init` requests
    /// subscriptions via the `subscribe` host import).
    pub fn subscribe(&mut self, topic: impl Into<String>) {
        self.subscriptions.push(topic.into());
    }

    pub fn provide_service<T: 'static>(&mut self, value: T) -> Result<(), String> {
        self.registry.provide(value)
    }

    pub fn consume_service<T: 'static>(&mut self) -> Option<T> {
        self.registry.consume()
    }

    /// Drains the topics requested via `subscribe` — the caller applies
    /// them to this module's `Endpoint` once it exists.
    pub fn into_subscriptions(self) -> Vec<String> {
        self.subscriptions
    }
}
