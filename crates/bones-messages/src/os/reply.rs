use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// An OS completion correlated by the host-stamped caller and its request ID.
/// Replies are broadcast; consumers filter by requester before matching an ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reply<'a> {
    /// Sender of the original request, stamped by the host.
    pub requester: &'a str,
    /// ID unique among that caller's outstanding requests.
    pub request_id: u32,
    /// Whether the action produced an answer.
    pub accepted: bool,
    /// Clipboard text, chosen path, or encoded fetch body.
    pub value: &'a str,
    /// Failure and remedy; empty for success or cancellation.
    pub error: &'a str,
}

impl Message for Reply<'_> {
    const TOPIC: &'static str = "os/reply";
}

impl EncodeMessage for Reply<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(self.requester)
            .u32(self.request_id)
            .u8(u8::from(self.accepted))
            .str(self.value)
            .str(self.error)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Reply<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let requester = reader.read_str()?;
        let request_id = reader.read_u32()?;
        let accepted = match reader.read_u8()? {
            0 => false,
            1 => true,
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "os reply accepted",
                    tag,
                })
            }
        };
        let value = reader.read_str()?;
        let error = reader.read_str()?;
        reader.finish()?;
        Ok(Self {
            requester,
            request_id,
            accepted,
            value,
            error,
        })
    }
}
