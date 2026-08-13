use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Published when two `EntityOp::Spawn`-created colliders start touching
/// (`rapier2d::CollisionEvent::Started`) — never for a tilemap collider on
/// either side, and never for separation (`Stopped` is not surfaced).
/// `entity_id_a`/`entity_id_b` are unordered: which is which depends only
/// on rapier2d's own internal collider ordering, not on which entity moved
/// into the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collision {
    pub entity_id_a: u32,
    pub entity_id_b: u32,
}

impl Message for Collision {
    const TOPIC: &'static str = "game-core/collision";
}

impl EncodeMessage for Collision {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.entity_id_a)
            .u32(self.entity_id_b)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Collision {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            entity_id_a: reader.read_u32()?,
            entity_id_b: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
