use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Sets a spawned entity's rapier2d linear velocity directly — the
/// mechanism a caller (an extension reading `input/*`) drives movement
/// with, rather than every frame re-deriving position itself. A no-op if
/// `entity_id` names an entity with no collider (nothing to set velocity
/// on) or no entity at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetVelocity {
    /// The `SpawnEntity::entity_id` this velocity applies to.
    pub entity_id: u32,
    pub vx: f32,
    pub vy: f32,
}

impl Message for SetVelocity {
    const TOPIC: &'static str = "game-core/set-velocity";
}

impl EncodeMessage for SetVelocity {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.entity_id)
            .f32(self.vx)
            .f32(self.vy)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for SetVelocity {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            entity_id: reader.read_u32()?,
            vx: reader.read_f32()?,
            vy: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
