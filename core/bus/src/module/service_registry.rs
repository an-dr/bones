use std::any::{Any, TypeId};
use std::collections::HashMap;

/// `TypeId`-keyed store for the services modules provide to and consume
/// from each other (`window-surface`, `draw-target`, design/modules.md).
/// Owned services retain single-consumer transfer semantics through
/// `consume`. `get` additionally permits build-time borrowing for consumers
/// such as web that only need to derive a native child handle before the
/// renderer takes ownership of the SDL window. Not `Send`-bounded:
/// `Engine::build` uses this synchronously on one thread, and some real
/// services (e.g. `sdl3::video::Window`) aren't `Send` themselves.
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
            return Err(format!(
                "service of type {} already provided",
                std::any::type_name::<T>()
            ));
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

    /// Borrows a provided service without claiming its ownership.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }
}
