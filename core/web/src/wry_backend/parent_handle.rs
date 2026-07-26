use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};

/// Borrowable parent handle retained while the SDL window remains alive.
pub(super) struct ParentHandle {
    raw: RawWindowHandle,
}

impl ParentHandle {
    pub(super) fn new(raw: RawWindowHandle) -> Self {
        Self { raw }
    }
}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: `WryBackend::new` obtains this handle from its live SDL
        // window, and the backend contract requires that window to outlive
        // every child webview created through this wrapper.
        Ok(unsafe { WindowHandle::borrow_raw(self.raw) })
    }
}
