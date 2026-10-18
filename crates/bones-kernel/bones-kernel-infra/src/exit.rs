//! The `exit-request` service: a one-way request to begin orderly shutdown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A shared flag any module may set to ask the kernel to shut down.
///
/// Cloning shares the flag. The kernel provides one as a service and checks
/// it once per frame; the extension host sets it when an extension calls
/// `request-exit`. There is no confirmation and no way to cancel: the kernel
/// begins its orderly shutdown on the next frame.
#[derive(Debug, Clone, Default)]
pub struct ExitRequest(Arc<AtomicBool>);

impl ExitRequest {
    /// A fresh, unset request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the kernel to shut down.
    pub fn request(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether anything has asked for shutdown.
    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
