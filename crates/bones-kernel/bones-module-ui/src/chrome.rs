//! The window's own title bar (ADR-049): the system frame is removed, and a
//! hit-test hands moving, resizing, and the desktop's snapping back to the
//! window manager. Installed the first time a `title-bar` widget is drawn.

use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use sdl3::sys::{events, rect::SDL_Point, video};

const RESIZE_EDGE: i32 = 6;

// Read by the hit-test, which SDL calls outside any frame. In window pixels.
static TITLE_PIXELS: AtomicU32 = AtomicU32::new(0);
static BUTTON_PIXELS: AtomicU32 = AtomicU32::new(0);

const NOT_TRIED: u8 = 0;
const INSTALLED: u8 = 1;
const REFUSED: u8 = 2;
static STATE: AtomicU8 = AtomicU8::new(NOT_TRIED);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Zone {
    Normal,
    Drag,
    Resize(i8, i8),
}

/// What a window point does. Caption buttons stay normal so egui receives
/// their clicks.
pub(crate) fn zone(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    title: i32,
    buttons: i32,
    maximized: bool,
) -> Zone {
    if !maximized {
        let horizontal = if x < RESIZE_EDGE {
            -1
        } else if x >= width - RESIZE_EDGE {
            1
        } else {
            0
        };
        let vertical = if y < RESIZE_EDGE {
            -1
        } else if y >= height - RESIZE_EDGE {
            1
        } else {
            0
        };
        if horizontal != 0 || vertical != 0 {
            return Zone::Resize(horizontal, vertical);
        }
    }
    if y < title && x < width - buttons {
        return Zone::Drag;
    }
    Zone::Normal
}

unsafe extern "C" fn hit_test(
    window: *mut video::SDL_Window,
    area: *const SDL_Point,
    _data: *mut std::ffi::c_void,
) -> video::SDL_HitTestResult {
    let (mut width, mut height) = (0, 0);
    // SAFETY: SDL passes a live window and a valid point for this call.
    let (point, maximized): (SDL_Point, bool) = unsafe {
        video::SDL_GetWindowSizeInPixels(window, &mut width, &mut height);
        (
            *area,
            video::SDL_GetWindowFlags(window).0 & video::SDL_WINDOW_MAXIMIZED.0 != 0,
        )
    };
    let title = TITLE_PIXELS.load(Ordering::Relaxed) as i32;
    let buttons = BUTTON_PIXELS.load(Ordering::Relaxed) as i32;
    match zone(point.x, point.y, width, height, title, buttons, maximized) {
        Zone::Normal => video::SDL_HITTEST_NORMAL,
        Zone::Drag => video::SDL_HITTEST_DRAGGABLE,
        Zone::Resize(-1, -1) => video::SDL_HITTEST_RESIZE_TOPLEFT,
        Zone::Resize(0, -1) => video::SDL_HITTEST_RESIZE_TOP,
        Zone::Resize(1, -1) => video::SDL_HITTEST_RESIZE_TOPRIGHT,
        Zone::Resize(1, 0) => video::SDL_HITTEST_RESIZE_RIGHT,
        Zone::Resize(1, 1) => video::SDL_HITTEST_RESIZE_BOTTOMRIGHT,
        Zone::Resize(0, 1) => video::SDL_HITTEST_RESIZE_BOTTOM,
        Zone::Resize(-1, 1) => video::SDL_HITTEST_RESIZE_BOTTOMLEFT,
        Zone::Resize(..) => video::SDL_HITTEST_RESIZE_LEFT,
    }
}

/// The engine's one window, which SDL lists once started.
fn window() -> *mut video::SDL_Window {
    let mut count = 0;
    // SAFETY: SDL returns a null-terminated list or null, which the caller frees.
    unsafe {
        let list = video::SDL_GetWindows(&mut count);
        if list.is_null() {
            return null_mut();
        }
        let first = if count > 0 { *list } else { null_mut() };
        sdl3::sys::stdinc::SDL_free(list.cast());
        first
    }
}

/// Replaces the system frame once; afterwards says whether that worked. On
/// failure the system frame stays, and no second title bar may be drawn.
pub(crate) fn ensure_installed() -> bool {
    match STATE.load(Ordering::Relaxed) {
        INSTALLED => return true,
        REFUSED => return false,
        _ => {}
    }
    let installed = install();
    STATE.store(
        if installed { INSTALLED } else { REFUSED },
        Ordering::Relaxed,
    );
    installed
}

fn install() -> bool {
    let window = window();
    if window.is_null() {
        return false;
    }
    // SAFETY: `window` is live, and this runs on the frame thread that owns it.
    unsafe {
        if !video::SDL_SetWindowHitTest(window, Some(hit_test), null_mut()) {
            return false;
        }
        if !video::SDL_SetWindowBordered(window, false) {
            video::SDL_SetWindowHitTest(window, None, null_mut());
            return false;
        }
    }
    true
}

/// Tells the hit-test where the bar and its buttons are, in window pixels.
pub(crate) fn publish_layout(title_pixels: f32, button_pixels: f32) {
    TITLE_PIXELS.store(title_pixels.max(0.0) as u32, Ordering::Relaxed);
    BUTTON_PIXELS.store(button_pixels.max(0.0) as u32, Ordering::Relaxed);
}

pub(crate) fn is_maximized() -> bool {
    let window = window();
    // SAFETY: `window` is checked and live on its own thread.
    !window.is_null()
        && unsafe { video::SDL_GetWindowFlags(window).0 & video::SDL_WINDOW_MAXIMIZED.0 != 0 }
}

pub(crate) fn minimize() {
    let window = window();
    if !window.is_null() {
        // SAFETY: as above.
        unsafe { video::SDL_MinimizeWindow(window) };
    }
}

pub(crate) fn toggle_maximize() {
    let (window, maximized) = (window(), is_maximized());
    if !window.is_null() {
        // SAFETY: as above.
        unsafe {
            if maximized {
                video::SDL_RestoreWindow(window);
            } else {
                video::SDL_MaximizeWindow(window);
            }
        };
    }
}

/// Asks for the same orderly shutdown as the system's close button.
pub(crate) fn close() {
    // SAFETY: an all-zero SDL_Event is valid once its type is set, and SDL copies it.
    unsafe {
        let mut event: events::SDL_Event = std::mem::zeroed();
        event.r#type = events::SDL_EVENT_QUIT.0;
        events::SDL_PushEvent(&mut event);
    }
}

#[cfg(test)]
mod tests;
