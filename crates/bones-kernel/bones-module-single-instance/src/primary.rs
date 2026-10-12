use std::sync::mpsc::{self, Receiver};

use crate::platform::Guard;
use crate::Handoff;

/// This process is the instance for its app_uid.
///
/// - Later launches arrive on [`Primary::incoming`], one [`Handoff`] each,
///   received on a listener thread this value owns.
/// - Dropping it releases the claim and stops the listener, so the next
///   launch becomes the instance.
/// - A primary that took over from an unresponsive instance is not listening:
///   the hung process still holds the endpoint, so this one starts as an
///   instance of its own and receives nothing.
pub struct Primary {
    incoming: Receiver<Handoff>,
    // Declared after `incoming` only for readability; the guard's drop is
    // what releases the endpoint, and nothing here depends on the order.
    guard: Option<Guard>,
}

impl Primary {
    pub(crate) fn listening(incoming: Receiver<Handoff>, guard: Guard) -> Self {
        Self {
            incoming,
            guard: Some(guard),
        }
    }

    /// A primary that holds no endpoint: its channel is closed from the start.
    pub(crate) fn detached() -> Self {
        let (_, incoming) = mpsc::channel();
        Self {
            incoming,
            guard: None,
        }
    }

    /// Each later launch, in the order they arrived. Drain it without
    /// blocking — `try_recv` — from the frame loop.
    pub fn incoming(&self) -> &Receiver<Handoff> {
        &self.incoming
    }

    /// Whether this process holds the claim and receives later launches.
    /// False only after falling back from an instance that did not answer.
    pub fn is_listening(&self) -> bool {
        self.guard.is_some()
    }
}

impl std::fmt::Debug for Primary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Primary")
            .field("listening", &self.is_listening())
            .finish()
    }
}
