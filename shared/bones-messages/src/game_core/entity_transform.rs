use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Authoritative world-space center of one caller-addressable game-core entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityTransform {
    pub entity_id: u32,
    pub x: f32,
    pub y: f32,
}

impl Message for EntityTransform {
    const TOPIC: &'static str = "game-core/entity-transform";
}

impl EncodeMessage for EntityTransform {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.entity_id)
            .f32(self.x)
            .f32(self.y)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for EntityTransform {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            entity_id: reader.read_u32()?,
            x: reader.read_f32()?,
            y: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
