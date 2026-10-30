use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Legacy completion without requester identity; prefer [`super::Reply`].
///
/// `accepted` separates "there is no answer" from "the answer is empty",
/// which the two string fields cannot do alone. A cancelled file dialog, a
/// URL holding nothing, and a missing file are all ordinary outcomes: they
/// answer `accepted: false` with an empty `error`. Only a genuine failure
/// carries `error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Result<'a> {
    /// The `request_id` of the request this answers.
    pub request_id: u32,
    /// Whether the action produced an answer.
    pub accepted: bool,
    /// The answer: clipboard text, a chosen path, or fetched text. Empty when there is none.
    pub value: &'a str,
    /// Why the action failed. Empty unless it did.
    pub error: &'a str,
}

impl Message for Result<'_> {
    const TOPIC: &'static str = "os/result";
}

impl EncodeMessage for Result<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(u8::from(self.accepted))
            .str(self.value)
            .str(self.error)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Result<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let accepted = match reader.read_u8()? {
            0 => false,
            1 => true,
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "os result accepted",
                    tag,
                })
            }
        };
        let value = reader.read_str()?;
        let error = reader.read_str()?;
        reader.finish()?;
        Ok(Self {
            request_id,
            accepted,
            value,
            error,
        })
    }
}
