use crate::bus::Handler;

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

    /// First look at one raw platform event, before it is translated onto
    /// `input/*` (ADR-008: the top layer consumes). Returning `true`
    /// claims the event, so no lower layer and no extension ever sees it —
    /// how a UI module stops a click on one of its widgets from also
    /// reaching the game underneath.
    ///
    /// - Default is a no-op claiming nothing, which is right for every
    ///   module that does not draw interactive surfaces.
    /// - Every module is offered the event in *reverse* registration order
    ///   until one claims it — the module registered last draws above the
    ///   rest, so it is asked first. A module that needs to *observe*
    ///   input it does not claim must still return `false`.
    /// - Feature-gated with `platform` because the parameter is the SDL
    ///   event itself: no translation layer sits in between, so nothing is
    ///   lost on the way (text input and modifier state especially, which
    ///   the `input/*` vocabulary does not carry). A headless build has no
    ///   event source and so does not have this method at all.
    #[cfg(feature = "platform")]
    fn filter_event(&mut self, _event: &super::PlatformEvent) -> bool {
        false
    }

    /// Called once per frame before `core/tick` is published and dispatched.
    ///
    /// For work the coming tick must already see — the extension host swaps
    /// in a rebuilt extension here, so its first tick runs the new code.
    /// Default is a no-op.
    fn before_tick(&mut self) {}

    /// Called once per frame after the tick's messages are dispatched and
    /// before `render`.
    ///
    /// For reacting to what that dispatch did — the extension host
    /// quarantines an extension that faulted during this tick here, this
    /// frame rather than the next. Default is a no-op.
    fn after_tick(&mut self) {}

    /// `render` phase: draw work that doesn't need to happen exactly at
    /// present time. Most modules don't need this; default is a no-op.
    fn render(&mut self) {}

    /// `present` phase: flip buffers / finalize the frame.
    fn present(&mut self) {}

    /// First step of orderly shutdown, after the close request is dispatched
    /// and before any module's `shutdown`.
    ///
    /// For stopping what other modules may still need to hear from: whatever
    /// this publishes is dispatched while every module is still running. The
    /// extension host calls each extension's own `shutdown` here. Default is
    /// a no-op.
    fn begin_shutdown(&mut self) {}

    /// Called once during orderly shutdown, after every module's
    /// `begin_shutdown` has run and its messages have been dispatched.
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
