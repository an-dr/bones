//! The `frame-control` service (design/modules.md): lets a module pause the
//! simulation and step it frame by frame, for a test or an agent that needs
//! repeatable observations.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Pause, resume, and step requests, shared between whoever asks and the
/// run loop that obeys. Cheap to clone; every clone is the same control.
#[derive(Clone, Default)]
pub struct FrameControl {
    inner: Arc<State>,
}

#[derive(Default)]
struct State {
    paused: AtomicBool,
    steps: AtomicU32,
}

impl FrameControl {
    /// A control that starts running.
    pub fn new() -> Self {
        Self::default()
    }

    /// Holds the simulation still from the next frame on.
    pub fn pause(&self) {
        self.inner.paused.store(true, Ordering::SeqCst);
    }

    /// Lets the simulation run again and drops any steps not yet taken.
    pub fn resume(&self) {
        self.inner.steps.store(0, Ordering::SeqCst);
        self.inner.paused.store(false, Ordering::SeqCst);
    }

    /// Asks for `frames` more frames while paused.
    pub fn step(&self, frames: u32) {
        self.inner.steps.fetch_add(frames, Ordering::SeqCst);
    }

    /// Whether the simulation is held still.
    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    /// Steps asked for and not yet taken.
    pub fn pending_steps(&self) -> u32 {
        self.inner.steps.load(Ordering::SeqCst)
    }

    /// The run loop's question for each frame: the `dt` to simulate with, or
    /// `None` to skip the simulation this frame.
    ///
    /// - Running: `measured`, the time since the last frame.
    /// - Paused with a step pending: `fixed`, the tick period, and one step
    ///   is used up, so stepped runs are repeatable.
    /// - Paused otherwise: `None`.
    pub fn next_dt(&self, measured: f32, fixed: f32) -> Option<f32> {
        if !self.is_paused() {
            return Some(measured);
        }
        // Takes one pending step, if any, even while another thread adds more.
        let steps = &self.inner.steps;
        let mut current = steps.load(Ordering::SeqCst);
        while current > 0 {
            match steps.compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return Some(fixed),
                Err(actual) => current = actual,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
