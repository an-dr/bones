use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::EntityOp;

/// Wire envelope for one `EntityOp` (ADR-019, open/closed: the tagged
/// `EntityOp` enum is what extends, not this topic).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityOpMessage(pub EntityOp);

impl Message for EntityOpMessage {
    const TOPIC: &'static str = "game-core/entity-op";
}

impl EncodeMessage for EntityOpMessage {
    fn encode(&self) -> Vec<u8> {
        self.0.encode_into(Writer::new()).finish()
    }
}

impl DecodeMessage<'_> for EntityOpMessage {
    fn decode(payload: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let op = EntityOp::decode(&mut reader)?;
        reader.finish()?;
        Ok(Self(op))
    }
}
