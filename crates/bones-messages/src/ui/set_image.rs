use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Uploads an image the sender's `image` widgets then show by `handle`
/// (ADR-048), so a tree never carries pixels. A second upload under the same
/// handle replaces the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetImage<'a> {
    /// The sender's own name for the image; handles of different owners
    /// never collide.
    pub handle: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height` pixels, row by row, as straight RGBA8.
    pub rgba: &'a [u8],
}

impl Message for SetImage<'_> {
    const TOPIC: &'static str = "ui/image";
}

impl EncodeMessage for SetImage<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.handle)
            .u32(self.width)
            .u32(self.height)
            .bytes(self.rgba)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for SetImage<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            handle: reader.read_u32()?,
            width: reader.read_u32()?,
            height: reader.read_u32()?,
            rgba: reader.read_rest(),
        })
    }
}
