//! The native module contract (design/modules.md, ADR-017): a bus endpoint
//! that additionally hooks the frame phases it needs and reaches sibling
//! capabilities through a typed service registry, instead of depending on
//! the provider's crate directly. Lives in `bus` (not `runner`, which
//! drives it) so a module crate never needs `runner` itself as a
//! dependency — only `bus`, which every module already depends on for
//! `Handler`.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::Handler;

/// A native module: a bus endpoint (`Handler`, structure.md: modules and
/// extensions are indistinguishable on the bus) that also hooks the
/// `render`/`present` frame phases it needs (design/modules.md's phase
/// table — `dispatch` and `tick` need no separate hook, they already ride
/// the bus). WASM extensions never implement this; see design/
/// extensions.md for their own contract.
pub trait Module: Handler {
    /// The bus endpoint name this module registers under.
    fn name(&self) -> &str;

    /// Called once at build time, in registration order. Requests bus
    /// subscriptions and provides/consumes services via `ctx` — mirrors
    /// how a WASM extension's own `init` requests subscriptions.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String>;

    /// `render` phase: draw work that doesn't need to happen exactly at
    /// present time. Most modules don't need this; default is a no-op.
    fn render(&mut self) {}

    /// `present` phase: flip buffers / finalize the frame.
    fn present(&mut self) {}

    /// TODO: not called by `Engine::run` yet — the full shutdown sequence
    /// (WIT `shutdown` export, close-request event, design/platform.md) is
    /// a separate roadmap rung.
    fn shutdown(&mut self) {}

    /// Answers a direct `send` (ADR-010) addressed to this module by name.
    ///
    /// - Same capability WASM extensions already have via the WIT `send`
    ///   import — modules and extensions stay indistinguishable on the bus
    ///   for direct calls too, not just pub/sub (design/modules.md).
    /// - Default: no reply, same as an extension whose `on-message`
    ///   returns `None`. Most modules (renderer, ui, audio) never override
    ///   this; `persistence` is the first that needs to.
    fn respond(&mut self, _sender: &str, _payload: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// `TypeId`-keyed store for the services modules provide to and consume
/// from each other (`window-surface`, `draw-target`, design/modules.md).
/// Single-consumer, ownership-transfer semantics — `consume` removes the
/// value, matching how `Platform::take_window` already works. TODO:
/// revisit (e.g. `Arc`-wrapped services) once a service needs more than
/// one consumer (modules.md lists `web` as a second `window-surface`
/// consumer, once it exists). Not `Send`-bounded: `Engine::build` uses this
/// synchronously on one thread, and some real services (e.g. `sdl3::video::
/// Window`) aren't `Send` themselves.
#[derive(Default)]
pub struct ServiceRegistry {
    services: HashMap<TypeId, Box<dyn Any>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` as the service of its own type. Errors if a
    /// service of the same type was already provided — provider/consumer
    /// registration order is a build-time contract (design/modules.md),
    /// not something to silently overwrite.
    pub fn provide<T: 'static>(&mut self, value: T) -> Result<(), String> {
        if self.services.contains_key(&TypeId::of::<T>()) {
            return Err(format!("service of type {} already provided", std::any::type_name::<T>()));
        }
        self.services.insert(TypeId::of::<T>(), Box::new(value));
        Ok(())
    }

    /// Takes the service of type `T` out of the registry, if one was
    /// provided — a second `consume::<T>()` call gets `None`.
    pub fn consume<T: 'static>(&mut self) -> Option<T> {
        self.services
            .remove(&TypeId::of::<T>())
            .map(|boxed| *boxed.downcast::<T>().expect("service TypeId mismatch"))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provide_then_consume_round_trips() {
        let mut registry = ServiceRegistry::new();
        registry.provide(42u32).unwrap();
        assert_eq!(registry.consume::<u32>(), Some(42));
    }

    #[test]
    fn consume_without_a_provider_is_none() {
        let mut registry = ServiceRegistry::new();
        assert_eq!(registry.consume::<u32>(), None);
    }

    #[test]
    fn consume_is_single_use() {
        let mut registry = ServiceRegistry::new();
        registry.provide("window".to_string()).unwrap();
        assert_eq!(registry.consume::<String>(), Some("window".to_string()));
        assert_eq!(registry.consume::<String>(), None);
    }

    #[test]
    fn providing_the_same_type_twice_is_an_error() {
        let mut registry = ServiceRegistry::new();
        registry.provide(1u32).unwrap();
        let err = registry.provide(2u32).unwrap_err();
        assert!(err.contains("already provided"), "unexpected error: {err}");
    }

    #[test]
    fn different_types_do_not_collide() {
        let mut registry = ServiceRegistry::new();
        registry.provide(7u32).unwrap();
        registry.provide("seven".to_string()).unwrap();
        assert_eq!(registry.consume::<u32>(), Some(7));
        assert_eq!(registry.consume::<String>(), Some("seven".to_string()));
    }

    #[test]
    fn context_collects_requested_subscriptions() {
        let mut registry = ServiceRegistry::new();
        let mut ctx = ModuleContext::new(&mut registry);
        ctx.subscribe("gfx/*");
        ctx.subscribe("core/tick");
        assert_eq!(ctx.into_subscriptions(), vec!["gfx/*", "core/tick"]);
    }

    #[test]
    fn context_forwards_service_provide_and_consume() {
        let mut registry = ServiceRegistry::new();
        {
            let mut ctx = ModuleContext::new(&mut registry);
            ctx.provide_service(99u32).unwrap();
        }
        let mut ctx = ModuleContext::new(&mut registry);
        assert_eq!(ctx.consume_service::<u32>(), Some(99));
    }
}
