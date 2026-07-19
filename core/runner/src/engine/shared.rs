use std::sync::{Arc, Mutex};

use bus::{Envelope, Handler};

/// Forwards bus deliveries to a `T` shared with `Engine` itself, for calls
/// outside normal message delivery — `renderer`'s `render`/`present` each
/// frame, `ui`'s `update`, a boxed `Module`'s own hooks. `renderer`/`ui`
/// stay their own typed `BuiltEngine` fields (not folded into the generic
/// `modules` list) purely because `ui` still direct-wires to `renderer`
/// (docs/structure.md) — everything about how either is built and driven
/// otherwise goes through the real `Module` trait, same as any
/// `.module(...)`-injected one.
pub(super) struct Shared<T: Handler>(pub(super) Arc<Mutex<T>>);

impl<T: Handler> Handler for Shared<T> {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}
