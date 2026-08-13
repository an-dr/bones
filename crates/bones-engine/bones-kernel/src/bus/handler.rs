use crate::bus::Envelope;

/// Delivered-envelope callback; test doubles and real dispatch targets
/// (host, a module) both just implement this. Only `Send` is required:
/// every `Handler` is reached through `Arc<Mutex<Adapter>>`, and
/// `Mutex<T>` is itself `Send + Sync` whenever `T: Send`.
pub trait Handler: Send {
    fn handle(&mut self, envelope: &Envelope);
}

impl<F: FnMut(&Envelope) + Send> Handler for F {
    fn handle(&mut self, envelope: &Envelope) {
        self(envelope);
    }
}
