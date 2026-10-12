use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::Outcome;

/// One request's outcome, correlated by its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Result<'a> {
    pub request_id: u32,
    pub outcome: Outcome<'a>,
}

impl Result<'_> {
    /// Encodes the result, or `None` when `previous` in
    /// [`Outcome::LastApplied`] is over 65535 bytes.
    ///
    /// - A version string that long is not a version, but it is read back
    ///   from disk rather than chosen by the engine.
    /// - So its size is reported here rather than asserted, as
    ///   [`EncodeMessage::encode`] does.
    pub fn try_encode(&self) -> Option<Vec<u8>> {
        let writer = Writer::new().u32(self.request_id);
        let writer = match self.outcome {
            Outcome::Available { version } => writer.u8(0).bytes(version.as_bytes()),
            Outcome::UpToDate => writer.u8(1),
            Outcome::Staged { version } => writer.u8(2).bytes(version.as_bytes()),
            Outcome::Installed { fresh } => writer.u8(3).u8(u8::from(fresh)),
            Outcome::LastApplied { previous, current } => writer
                .u8(4)
                .u8(u8::from(previous.is_some()))
                .try_str(previous.unwrap_or_default())
                .ok()?
                .bytes(current.as_bytes()),
            Outcome::Failed { reason } => writer.u8(5).bytes(reason.as_bytes()),
        };
        Some(writer.finish())
    }
}

impl Message for Result<'_> {
    const TOPIC: &'static str = "upgrader/result";
}

impl EncodeMessage for Result<'_> {
    fn encode(&self) -> Vec<u8> {
        self.try_encode()
            .expect("previous version exceeds what upgrader/result can frame")
    }
}

impl<'a> DecodeMessage<'a> for Result<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let outcome = match reader.read_u8()? {
            0 => Outcome::Available {
                version: reader.read_str_rest()?,
            },
            1 => {
                reader.finish()?;
                Outcome::UpToDate
            }
            2 => Outcome::Staged {
                version: reader.read_str_rest()?,
            },
            3 => {
                let fresh = reader.read_u8()? != 0;
                reader.finish()?;
                Outcome::Installed { fresh }
            }
            4 => {
                let has_previous = reader.read_u8()? != 0;
                let previous = reader.read_str()?;
                Outcome::LastApplied {
                    previous: has_previous.then_some(previous),
                    current: reader.read_str_rest()?,
                }
            }
            5 => Outcome::Failed {
                reason: reader.read_str_rest()?,
            },
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "upgrader result",
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
