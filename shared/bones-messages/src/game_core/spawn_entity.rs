use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Spawns one entity with a world-space transform, a sprite to draw, and
/// (if `frame_count > 1`) a looping frame-index-from-elapsed-time
/// animation over that sprite's source rectangle.
// No `Eq`: every field but the counts is `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnEntity {
    /// Application-assigned entity identifier — its own namespace, separate
    /// from `sprite_id`. Lets a later `game-core/set-velocity` address this
    /// entity without the caller needing an ECS-internal handle.
    pub entity_id: u32,
    /// Application-assigned sprite identifier (matches `gfx::LoadSprite`).
    pub sprite_id: u32,
    /// World x coordinate.
    pub x: f32,
    /// World y coordinate.
    pub y: f32,
    /// Sprite frame width, in source pixels — every frame is laid out at
    /// `(frame_index * frame_w, 0)` in the source sprite.
    pub frame_w: u32,
    /// Sprite frame height, in source pixels.
    pub frame_h: u32,
    /// Total frames in the animation loop. `1` draws a static sprite with
    /// no timer.
    pub frame_count: u32,
    /// Seconds each frame is shown before advancing.
    pub frame_duration: f32,
    /// Half-width of a dynamic box collider. `0.0` (with `collider_half_h`)
    /// spawns the entity with no physics body at all — a purely visual
    /// entity, the common case for background dressing.
    pub collider_half_w: f32,
    /// Half-height of a dynamic box collider.
    pub collider_half_h: f32,
}

impl Message for SpawnEntity {
    const TOPIC: &'static str = "game-core/spawn-entity";
}

impl EncodeMessage for SpawnEntity {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.entity_id)
            .u32(self.sprite_id)
            .f32(self.x)
            .f32(self.y)
            .u32(self.frame_w)
            .u32(self.frame_h)
            .u32(self.frame_count)
            .f32(self.frame_duration)
            .f32(self.collider_half_w)
            .f32(self.collider_half_h)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for SpawnEntity {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            entity_id: reader.read_u32()?,
            sprite_id: reader.read_u32()?,
            x: reader.read_f32()?,
            y: reader.read_f32()?,
            frame_w: reader.read_u32()?,
            frame_h: reader.read_u32()?,
            frame_count: reader.read_u32()?,
            frame_duration: reader.read_f32()?,
            collider_half_w: reader.read_f32()?,
            collider_half_h: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}
