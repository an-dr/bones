use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};
use sdl3::video::Window;

/// Shared owner of the SDL parent window used by wry child views.
pub(super) struct ParentHandle {
    window: Window,
}

impl ParentHandle {
    pub(super) fn new(window: &Window) -> Self {
        Self {
            window: window.clone(),
        }
    }

    pub(super) fn pixel_size(&self) -> (u32, u32) {
        self.window.size_in_pixels()
    }
}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.window.window_handle()
    }
}
