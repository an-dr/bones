use std::sync::{Arc, Mutex};

use bones_kernel::bus::{Module, ModuleContext, Registry, ServiceRegistry};

use super::shared_module::SharedModule;

/// Runs `module.init`, registers it on the bus under its own name and in
/// the call `Registry` under the same name (`Module::respond`, ADR-010 —
/// most modules never override it, so this is a harmless no-reply
/// registration for them), and applies the subscriptions it requested —
/// the same init-then-subscribe sequencing `attach_extension` uses for
/// WASM extensions (ADR-017 mirrors that contract for native modules).
pub(super) fn register_module(
    bus: &bones_kernel::bus::Bus,
    registry: &Registry,
    services: &mut ServiceRegistry,
    modules: &mut Vec<Arc<Mutex<Box<dyn Module>>>>,
    mut module: Box<dyn Module>,
) -> Result<(), String> {
    let topics = {
        let mut ctx = ModuleContext::new(services);
        module.init(&mut ctx)?;
        ctx.into_subscriptions()
    };
    let name = module.name().to_string();
    let shared = Arc::new(Mutex::new(module));
    let ep = bus.register(name.clone(), SharedModule(shared.clone()));
    for topic in topics {
        ep.subscribe(topic);
    }
    registry.insert(name, Arc::new(SharedModule(shared.clone())));
    modules.push(shared);
    Ok(())
}
