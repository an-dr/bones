//! The `window-control` service (design/modules.md, ADR-035): acting on the
//! window without owning it.
//!
//! - `window-surface` is the SDL window itself. The renderer consumes it, and
//!   it cannot leave the frame thread, so a module that only needs to bring
//!   the window forward cannot hold it.
//! - This carries the capability instead: a cloneable handle, read with
//!   `get` and never consumed, so any number of modules can keep one.
//! - It lives in the always-present tier for the reason `draw-target` does:
//!   its consumer must not depend on the crate that provides it. Nothing here
//!   is SDL-specific, so a test or a headless host supplies its own.

use std::sync::Arc;

type Raise = dyn Fn() -> Result<(), String> + Send + Sync;

/// Brings the application's window to the front.
///
/// Calls must come from the frame thread, where every module hook already
/// runs: the platform's own implementation talks to the windowing system,
/// which accepts calls from that thread only.
#[derive(Clone)]
pub struct WindowControl {
    raise: Arc<Raise>,
}

impl WindowControl {
    /// A handle whose [`WindowControl::raise`] runs `raise`.
    ///
    /// The platform builds the real one; a test passes a closure that
    /// records the call.
    pub fn new(raise: impl Fn() -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self {
            raise: Arc::new(raise),
        }
    }

    /// Un-minimises, raises, and focuses the window, where the platform
    /// allows it.
    ///
    /// Windows refuses to move focus to a process the user is not already
    /// using unless that process was granted it, which is why a second launch
    /// grants the running instance focus before handing over (ADR-035). A
    /// refusal is not an error: the window still restores and raises.
    pub fn raise(&self) -> Result<(), String> {
        (self.raise)()
    }
}

impl std::fmt::Debug for WindowControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WindowControl")
    }
}
