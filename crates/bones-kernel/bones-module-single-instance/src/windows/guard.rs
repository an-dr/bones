use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use super::pipe;

/// Holds the claim while it lives; dropping it stops the listener and
/// releases the pipe.
pub(crate) struct Guard {
    name: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Guard {
    pub(super) fn new(name: String, stop: Arc<AtomicBool>, thread: JoinHandle<()>) -> Self {
        Self {
            name,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // The listener blocks in ConnectNamedPipe; a connection of our own
        // is what wakes it to see the flag.
        let _ = pipe::connect(&self.name, Duration::from_secs(1));
        // Joined so the pipe is gone by the time this returns: a launch
        // right after must claim, not find a pipe that is shutting down.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
