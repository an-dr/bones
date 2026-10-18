//! The `display-info` service: what the window's display offers, queried once
//! before any window hand-off.

/// Display capability info queried once by `crate::platform::Platform` (before
/// any window hand-off) — small, read-only, and static for the process's
/// lifetime, so every consumer takes its own copy.
///
/// The kernel provides it as a service, so any module can read it with
/// `get_service`; the extension host hands it to every extension's
/// `list-display-modes` and `native-display-mode` calls.
#[derive(Debug, Clone, Default)]
pub struct DisplayInfo {
    /// Every fullscreen-capable resolution, `(width, height)`.
    pub modes: Vec<(u32, u32)>,
    /// The desktop's current resolution, if the query succeeded.
    pub native: Option<(u32, u32)>,
}
