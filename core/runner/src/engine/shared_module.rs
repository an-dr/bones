use std::sync::{Arc, Mutex};

use bus::{Envelope, Handler, Module, Respond};

/// Same shape as `Shared<T>`, but for a boxed `Module`: `Box<dyn Module>`
/// can't satisfy `Shared<T>`'s `T: Handler` bound generically (it would
/// need `impl Handler for Box<T>`, which conflicts with `bus`'s existing
/// blanket impl for `FnMut` closures — a coherence conflict, not a design
/// choice) — method-call syntax finds `handle` through auto-deref instead.
pub(super) struct SharedModule(pub(super) Arc<Mutex<Box<dyn Module>>>);

impl Handler for SharedModule {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

/// `Mutex<T>` is `Sync` whenever `T: Send` (locking supplies the missing
/// synchronization itself), so this needs nothing beyond the `Send` that
/// `Handler` (a `Module` supertrait) already requires — not `Module: Sync`
/// itself, which `Renderer`'s deliberately-non-`Sync` `SendWrapper` (see
/// its own doc comment) could never satisfy.
impl Respond for SharedModule {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().respond(sender, payload)
    }
}
