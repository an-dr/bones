use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Asks the renderer for an image of the next frame it presents; the answer
/// is a [`super::Captured`] carrying the same `request_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capture {
    /// Chosen by the caller, echoed on the reply.
    pub request_id: u32,
    /// What to capture.
    pub scope: Scope,
}

/// What a capture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// What the renderer drew: `gfx/*` and egui, read back before the frame
    /// is shown. Works everywhere, an offscreen window included.
    Canvas,
    /// The whole window as it appears on screen once the frame is shown,
    /// web panels included. Needs a visible window, on Windows or X11.
    Window,
}

impl Scope {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Scope::Canvas => 0,
            Scope::Window => 1,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Scope::Canvas),
            1 => Ok(Scope::Window),
            tag => Err(DecodeError::InvalidTag {
                message: "capture scope",
                tag,
            }),
        }
    }
}

impl Message for Capture {
    const TOPIC: &'static str = "renderer/capture";
}

impl EncodeMessage for Capture {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(self.scope.tag())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Capture {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            request_id: reader.read_u32()?,
            scope: Scope::from_tag(reader.read_u8()?)?,
        };
        reader.finish()?;
        Ok(message)
    }
}
