use std::path::Path;

use crate::Error;

/// One frame the renderer presented, as a PNG file and its decoded pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// The frame as a PNG file, as the renderer encoded it.
    pub png: Vec<u8>,
    /// Tightly packed RGBA rows, top row first.
    rgba: Vec<u8>,
}

impl Screenshot {
    /// Decodes the renderer's PNG; fails when it is not an 8-bit RGBA image
    /// of the stated size.
    pub(crate) fn from_png(width: u32, height: u32, png: Vec<u8>) -> Result<Self, Error> {
        let decoder = png::Decoder::new(std::io::Cursor::new(&png));
        let mut reader = decoder
            .read_info()
            .map_err(|error| format!("decoding the screenshot: {error}"))?;
        let mut rgba = vec![0; reader.output_buffer_size().unwrap_or(0)];
        let info = reader
            .next_frame(&mut rgba)
            .map_err(|error| format!("decoding the screenshot: {error}"))?;
        let rgba8 =
            info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight;
        if !rgba8 || (info.width, info.height) != (width, height) {
            return Err(format!(
                "the screenshot is {}x{} {:?} {:?}, expected {width}x{height} 8-bit RGBA",
                info.width, info.height, info.color_type, info.bit_depth
            ));
        }
        rgba.truncate(info.buffer_size());
        Ok(Self {
            width,
            height,
            png,
            rgba,
        })
    }

    /// The `[r, g, b, a]` of the pixel at `(x, y)`, from the top left, or
    /// `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = ((y * self.width + x) * 4) as usize;
        self.rgba.get(at..at + 4)?.try_into().ok()
    }

    /// Writes the PNG to `path`, for a document or for a person to look at.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        std::fs::write(path, &self.png)
            .map_err(|error| format!("writing {}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests;
