use std::sync::{Arc, Mutex};

use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext};

use crate::recording::Recording;

/// A module subscribed to every topic, handing what it hears to a
/// [`Recording`] shared with [`crate::App`].
pub(crate) struct Recorder {
    pub(crate) recording: Arc<Mutex<Recording>>,
}

impl Handler for Recorder {
    fn handle(&mut self, envelope: &Envelope) {
        self.recording.lock().unwrap().hear(envelope);
    }
}

impl Module for Recorder {
    fn name(&self) -> &str {
        "bones-test"
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe("*");
        Ok(())
    }
}
