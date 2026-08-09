use std::sync::{Arc, Mutex};

use crate::{
    Bus, Endpoint, Envelope, Handler, Module, ModuleContext, Registry, Respond, ServiceRegistry,
};

// `SharedModule` stays with `ModuleRegistration`: it is only the synchronized
// bus/direct-call adapter for the registration's boxed module.
struct SharedModule(Arc<Mutex<Box<dyn Module>>>);

impl Handler for SharedModule {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

impl Respond for SharedModule {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().respond(sender, payload)
    }
}

/// A native module attached to a live bus and direct-call registry.
///
/// Detaching removes both registrations and runs module shutdown. This is the
/// runtime counterpart to build-time engine module registration.
pub struct ModuleRegistration {
    bus: Bus,
    registry: Registry,
    endpoint: Option<Endpoint>,
    module: Option<Arc<Mutex<Box<dyn Module>>>>,
}

impl ModuleRegistration {
    /// Initializes and attaches one module, rejecting an occupied endpoint.
    pub fn attach(
        bus: Bus,
        registry: Registry,
        services: &mut ServiceRegistry,
        mut module: impl Module + 'static,
    ) -> Result<Self, String> {
        let name = module.name().to_string();
        if registry.contains(&name) {
            return Err(format!("module endpoint '{name}' is already registered"));
        }
        let topics = {
            let mut context = ModuleContext::new(services);
            module.init(&mut context)?;
            context.into_subscriptions()
        };
        let module: Arc<Mutex<Box<dyn Module>>> = Arc::new(Mutex::new(Box::new(module)));
        let endpoint = bus.register(name.clone(), SharedModule(Arc::clone(&module)));
        for topic in topics {
            endpoint.subscribe(topic);
        }
        if !registry.try_insert(name.clone(), Arc::new(SharedModule(Arc::clone(&module)))) {
            bus.unregister(&endpoint);
            return Err(format!("module endpoint '{name}' is already registered"));
        }
        Ok(Self {
            bus,
            registry,
            endpoint: Some(endpoint),
            module: Some(module),
        })
    }

    /// Runs the module's render phase while it is attached.
    pub fn render(&mut self) {
        if let Some(module) = &self.module {
            module.lock().unwrap().render();
        }
    }

    /// Runs the module's present phase while it is attached.
    pub fn present(&mut self) {
        if let Some(module) = &self.module {
            module.lock().unwrap().present();
        }
    }

    /// Reports whether this registration still owns its endpoint.
    pub fn is_attached(&self) -> bool {
        self.module.is_some()
    }

    /// Shuts down and removes the module. Safe to call repeatedly.
    pub fn detach(&mut self) {
        let (Some(module), Some(endpoint)) = (self.module.take(), self.endpoint.take()) else {
            return;
        };
        module.lock().unwrap().shutdown();
        self.registry.remove(endpoint.name());
        self.bus.unregister(&endpoint);
        drop(endpoint);
        drop(module);
    }
}

impl Drop for ModuleRegistration {
    fn drop(&mut self) {
        self.detach();
    }
}

#[cfg(test)]
mod tests;
