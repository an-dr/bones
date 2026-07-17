//! `core/tick` (messaging.md): frame tick with delta time. The one place
//! the wire format is defined — both the host (runner, host) and any
//! guest that wants to read it agree on this layout.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// One frame tick with its delta time in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// Elapsed seconds since the preceding tick.
    pub dt: f32,
}

impl Message for Tick {
    const TOPIC: &'static str = "core/tick";
}

impl EncodeMessage for Tick {
    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.dt).finish()
    }
}

impl<'a> DecodeMessage<'a> for Tick {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            dt: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reads_back_what_encode_wrote() {
        let tick = Tick { dt: 1.0 / 60.0 };
        assert_eq!(Tick::decode(&tick.encode()), Ok(tick));
    }

    #[test]
    fn decode_rejects_the_wrong_byte_count() {
        assert_eq!(Tick::decode(&[1, 2, 3]), Err(DecodeError::Truncated));
        assert_eq!(Tick::decode(&[0; 5]), Err(DecodeError::TrailingBytes));
    }
}
