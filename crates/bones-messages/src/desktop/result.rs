use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::Outcome;

/// One request's outcome, correlated by its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Result<'a> {
    pub request_id: u32,
    pub outcome: Outcome<'a>,
}

impl Message for Result<'_> {
    const TOPIC: &'static str = "desktop/result";
}

impl EncodeMessage for Result<'_> {
    fn encode(&self) -> Vec<u8> {
        let writer = Writer::new().u32(self.request_id);
        match self.outcome {
            Outcome::Registered(registered) => writer.u8(0).u8(u8::from(registered)),
            Outcome::Failed(reason) => writer.u8(1).bytes(reason.as_bytes()),
        }
        .finish()
    }
}

impl<'a> DecodeMessage<'a> for Result<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let outcome = match reader.read_u8()? {
            0 => {
                let registered = reader.read_u8()? != 0;
                reader.finish()?;
                Outcome::Registered(registered)
            }
            1 => Outcome::Failed(reader.read_str_rest()?),
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "desktop result",
                    tag,
                })
            }
        };
        Ok(Self {
            request_id,
            outcome,
        })
    }
}
