/// Physical pixels per interface point for the focused window, or else the
/// primary display. 1 on Wayland, where the compositor already scales the
/// window, and whenever the query fails.
pub(crate) fn display_scale() -> f32 {
    use sdl3::sys::{keyboard, video};
    // SAFETY: called on the frame thread, from the ui module's render phase,
    // after the kernel has started SDL's video system. A null window or a
    // failed query is handled below, and the driver name is a NUL-terminated
    // string SDL owns.
    unsafe {
        let driver = video::SDL_GetCurrentVideoDriver();
        if !driver.is_null() && std::ffi::CStr::from_ptr(driver).to_bytes() == b"wayland" {
            return 1.0;
        }
        let window = keyboard::SDL_GetKeyboardFocus();
        let scale = if window.is_null() {
            video::SDL_GetDisplayContentScale(video::SDL_GetPrimaryDisplay())
        } else {
            video::SDL_GetWindowDisplayScale(window)
        };
        if scale > 0.0 {
            scale
        } else {
            1.0
        }
    }
}
