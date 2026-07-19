use crate::Handler;

use super::ModuleContext;

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
