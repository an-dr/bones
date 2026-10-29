use super::Scope;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// The renderer's answer to a [`super::Capture`]: the frame as a PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Captured<'a> {
    /// The request this answers.
    pub request_id: u32,
    /// What the image shows. A window capture the platform cannot make is
    /// answered with the canvas, and says so here.
    pub scope: Scope,
    /// Image width in physical pixels; `0` when the capture failed.
    pub width: u32,
    /// Image height in physical pixels; `0` when the capture failed.
    pub height: u32,
    /// The frame as a PNG file; empty when the capture failed.
    pub png: &'a [u8],
}

impl Message for Captured<'_> {
    const TOPIC: &'static str = "renderer/captured";
}

impl EncodeMessage for Captured<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(self.scope.tag())
            .u32(self.width)
            .u32(self.height)
            .bytes(self.png)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Captured<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            request_id: reader.read_u32()?,
            scope: Scope::from_tag(reader.read_u8()?)?,
            width: reader.read_u32()?,
            height: reader.read_u32()?,
            png: reader.read_rest(),
        })
    }
}
