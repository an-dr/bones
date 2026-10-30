use std::sync::{Arc, Mutex};

use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext};

/// Every send a stub answered, as `(sender, payload)`, shared by its clones.
type Received = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A stand-in native module: answers every direct send to `name` with one
/// fixed reply and keeps what was sent, so a component whose `init` sends a
/// module the test does not compose still loads.
///
/// Register it with `Kernel::module` and pass that kernel to
/// [`App::with_kernel`](crate::App::with_kernel). Clone it first to read
/// [`received`](Stub::received) afterwards: clones share one record.
#[derive(Clone)]
pub struct Stub {
    name: String,
    reply: Vec<u8>,
    received: Received,
}

impl Stub {
    /// An endpoint called `name` that answers with an empty payload.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reply: Vec::new(),
            received: Arc::default(),
        }
    }

    /// Answers every send with `reply` instead.
    pub fn reply(mut self, reply: impl Into<Vec<u8>>) -> Self {
        self.reply = reply.into();
        self
    }

    /// Every send so far, oldest first, as `(sender, payload)`.
    pub fn received(&self) -> Vec<(String, Vec<u8>)> {
        self.received.lock().unwrap().clone()
    }
}

impl Handler for Stub {
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for Stub {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self, _context: &mut ModuleContext) -> Result<(), String> {
        Ok(())
    }

    fn respond(&mut self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        self.received
            .lock()
            .unwrap()
            .push((sender.to_string(), payload.to_vec()));
        Some(self.reply.clone())
    }
}
