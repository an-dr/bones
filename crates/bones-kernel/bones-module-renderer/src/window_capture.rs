//! Reads the whole window back as the operating system shows it, child
//! windows included, for a `renderer/capture` of the window scope. Web panels
//! are child windows, so this is the only capture that contains them.
//!
//! - Windows and X11 read the window back from the system. Under a Wayland
//!   session, web panels put the window on X11 through Xwayland (the engine
//!   selects that driver when panels are composed), so the same X11 path
//!   reads it.
//! - A native Wayland window cannot hold web panels, and Wayland lets no
//!   client read a window back, so its window is exactly the canvas.
//! - Elsewhere, and for an offscreen window, it fails with the reason, and
//!   the renderer answers with the canvas.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use sdl3::video::Window;

use crate::frame_capture::encode_png;

#[cfg(windows)]
mod win32;
#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "ios"),
    not(target_os = "android")
))]
mod x11;

/// What a window capture produced.
pub(crate) enum WindowImage {
    /// The window as the system shows it, as `(width, height, png)`.
    Read(u32, u32, Vec<u8>),
    /// The window holds nothing but the canvas, so the canvas image is the
    /// window image.
    SameAsCanvas,
}

/// The window as shown. Call after `present`, so the frame just drawn is
/// the one on screen.
pub(crate) fn capture_window(window: &Window) -> Result<WindowImage, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("the window has no native handle ({error})"))?;
    let (width, height, rgba) = match handle.as_raw() {
        RawWindowHandle::Wayland(_) => return Ok(WindowImage::SameAsCanvas),
        #[cfg(windows)]
        RawWindowHandle::Win32(handle) => win32::capture(handle.hwnd.get())?,
        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        ))]
        RawWindowHandle::Xlib(handle) => x11::capture(handle.window as u32)?,
        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        ))]
        RawWindowHandle::Xcb(handle) => x11::capture(handle.window.get())?,
        other => {
            return Err(format!(
                "capturing a {} window is not supported",
                kind(&other)
            ))
        }
    };
    Ok(WindowImage::Read(
        width,
        height,
        encode_png(width, height, &rgba)?,
    ))
}

/// The windowing system's name, for the error.
fn kind(handle: &RawWindowHandle) -> String {
    let debug = format!("{handle:?}");
    debug.split('(').next().unwrap_or("native").to_string()
}

/// `[b, g, r, x]` pixels as `[r, g, b, 255]`, the layout both Windows DIBs
/// and 32-bit little-endian X images use.
fn bgrx_to_rgba(bgrx: &mut [u8]) {
    for pixel in bgrx.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
}

#[cfg(test)]
mod tests;
