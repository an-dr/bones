//! The native module contract (design/modules.md, ADR-017): a bus endpoint
//! that additionally hooks the frame phases it needs and reaches sibling
//! capabilities through a typed service registry, instead of depending on
//! the provider's crate directly. Lives in `bus` (not `runner`, which
//! drives it) so a module crate never needs `runner` itself as a
//! dependency — only `bus`, which every module already depends on for
//! `Handler`.

mod module_context;
mod module_registration;
mod module_trait;
#[cfg(feature = "platform")]
mod offer_event;
mod service_registry;

pub use module_context::ModuleContext;
pub use module_registration::ModuleRegistration;
pub use module_trait::Module;
#[cfg(feature = "platform")]
pub use offer_event::offer_event;
pub use service_registry::ServiceRegistry;

#[cfg(test)]
mod tests;
