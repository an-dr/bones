//! The window through the X server's `GetImage`, which returns what the
//! window shows, mapped child windows included. A separate connection is
//! enough: window ids are global to the display.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, ImageOrder};

use super::bgrx_to_rgba;

/// Window `window` on `$DISPLAY` as `(width, height, rgba)`. Only the common
/// 32-bit little-endian pixel layout is read; anything else is an error.
pub(super) fn capture(window: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let failed = |error: &dyn std::fmt::Display| format!("X11: {error}");
    let (connection, _) = x11rb::connect(None).map_err(|e| failed(&e))?;
    let geometry = connection
        .get_geometry(window)
        .map_err(|e| failed(&e))?
        .reply()
        .map_err(|e| failed(&e))?;
    let image = connection
        .get_image(
            ImageFormat::Z_PIXMAP,
            window,
            0,
            0,
            geometry.width,
            geometry.height,
            u32::MAX,
        )
        .map_err(|e| failed(&e))?
        .reply()
        .map_err(|e| failed(&e))?;
    let setup = connection.setup();
    let bits = setup
        .pixmap_formats
        .iter()
        .find(|format| format.depth == image.depth)
        .map(|format| format.bits_per_pixel);
    if bits != Some(32) || setup.image_byte_order != ImageOrder::LSB_FIRST {
        return Err(format!(
            "X11: a depth-{} image with {bits:?} bits per pixel is not read",
            image.depth
        ));
    }
    let (width, height) = (u32::from(geometry.width), u32::from(geometry.height));
    let mut pixels = image.data;
    pixels.truncate(width as usize * height as usize * 4);
    bgrx_to_rgba(&mut pixels);
    Ok((width, height, pixels))
}
