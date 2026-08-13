mod adapter;
mod budget;
// docs/code-style.md names each file after the single type it holds, so the
// module that holds `Bus` is `bus/bus.rs` inside `bus/`. The lint is right in
// general and wrong against a convention that is repository-wide and applied
// to every other type here.
#[allow(
    clippy::module_inception,
    reason = "one type per file, named after the type (docs/code-style.md)"
)]
mod bus;
mod endpoint;
mod envelope;
mod handler;
mod module;
mod registry;
mod respond;
mod send_error;

pub use budget::{BudgetLimits, DropCounters, EndpointBudget};
pub use bus::Bus;
pub use endpoint::Endpoint;
pub use envelope::Envelope;
pub use handler::Handler;
#[cfg(feature = "platform")]
pub use module::{offer_event, PlatformEvent};
pub use module::{Module, ModuleContext, ModuleRegistration, ServiceRegistry};
pub use registry::Registry;
pub use respond::Respond;
pub use send_error::SendError;

#[cfg(test)]
mod tests;
