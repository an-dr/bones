use std::sync::{Arc, Mutex};

use bus::{Envelope, Handler, Respond};
use wasm_extensions::host::Host;

/// One extension `Host`, shared between its `Bus` registration (pub/sub
/// delivery), its `Registry` registration (direct send, ADR-010), and the
/// `Supervisor` (which needs `Host::is_faulted` after every call to know
/// when to quarantine it) — all three need the same instance, not
/// independent copies, so state stays consistent across all of them.
#[derive(Clone)]
pub(crate) struct SharedHost(pub(super) Arc<Mutex<Host>>);

impl Handler for SharedHost {
    fn handle(&mut self, envelope: &Envelope) {
        self.0.lock().unwrap().handle(envelope);
    }
}

impl Respond for SharedHost {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().unwrap().respond(sender, payload)
    }
}

impl SharedHost {
    pub(crate) fn is_faulted(&self) -> bool {
        self.0.lock().unwrap().is_faulted()
    }

    pub(crate) fn shutdown(&self) -> wasmtime::Result<()> {
        self.0.lock().unwrap().shutdown()
    }
}
