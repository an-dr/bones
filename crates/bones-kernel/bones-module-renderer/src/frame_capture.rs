//! Reads the frame the canvas holds back as a PNG, for `renderer/capture`.

use sdl3::pixels::PixelFormat;
use sdl3::render::Canvas;
use sdl3::video::Window;

/// The frame drawn so far, as `(width, height, png)`.
///
/// Must run after every module has drawn and before the canvas presents:
/// what the back buffer holds after `present` is undefined.
pub(crate) fn capture_png(canvas: &Canvas<Window>) -> Result<(u32, u32, Vec<u8>), String> {
    let surface = canvas
        .read_pixels(None)
        .map_err(|error| format!("reading the frame: {error}"))?
        .convert_format(PixelFormat::RGBA32)
        .map_err(|error| format!("converting the frame: {error}"))?;
    let (width, height, pitch) = (surface.width(), surface.height(), surface.pitch());
    // Rows may be padded beyond `width * 4` bytes; the PNG takes them tight.
    let rgba = surface.with_lock(|pixels| {
        pixels
            .chunks(pitch as usize)
            .take(height as usize)
            .flat_map(|row| &row[..width as usize * 4])
            .copied()
            .collect::<Vec<u8>>()
    });
    Ok((width, height, encode_png(width, height, &rgba)?))
}

/// Encodes tightly packed RGBA rows as a PNG file.
pub(crate) fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("encoding the frame: {error}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|error| format!("encoding the frame: {error}"))?;
    writer
        .finish()
        .map_err(|error| format!("encoding the frame: {error}"))?;
    Ok(png)
}

#[cfg(test)]
mod tests;
