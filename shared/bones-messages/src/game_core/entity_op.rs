use crate::{DecodeError, Reader, Writer};

/// The rapier2d rigid-body type a spawned entity's collider gets, when it
/// has one at all (`collider_half_w`/`collider_half_h` both `> 0.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    /// Pushed by other bodies and gravity, participates fully in rapier2d's
    /// collision response — the default, and every existing caller's
    /// behavior before `BodyKind` existed.
    #[default]
    Dynamic,
    /// Moves exactly as `SetVelocity` commands it and pushes `Dynamic`
    /// bodies out of its way on contact, but is never itself pushed —
    /// rapier2d's standard "platform/mover" body type (`kinematic` in
    /// Unity, Godot, and Box2D alike).
    Kinematic,
    /// A `Dynamic` body (pushed by contact, participates in collision
    /// response) with very high linear damping and rotation locked: it
    /// moves when pushed, but carries no momentum — velocity decays to
    /// rest almost immediately once nothing is pushing it, instead of
    /// coasting or drifting, and it never spins from an off-center hit.
    Frictionless,
}

/// The drawable appearance of a spawned entity: either an animated sprite
/// or a plain filled square at the collider's extent (obstacles, walls —
/// anything that doesn't need art). Exactly one of these two shapes per
/// entity; `EntityOp::Spawn` carries `Option<Sprite>` plus a separate
/// `square_color`, one of which is expected to be meaningful depending on
/// whether `sprite` is `Some`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    /// Application-assigned sprite identifier (matches `gfx::LoadSprite`).
    pub sprite_id: u32,
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
}

/// One operation on a `game-core` entity, addressed by a caller-assigned
/// `entity_id` (its own namespace, distinct from `gfx::LoadSprite`'s
/// sprite ids and this crate's other ids). Open/closed: a new op extends
/// this enum — the wire format inside `game-core/entity-op` — rather than
/// adding a new bus topic, the same pattern `ui::Widget` already uses for
/// `ui/spec`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityOp {
    /// Spawns an entity with a world-space transform and either an
    /// animated `sprite` or a plain filled `square_color` square at the
    /// collider's extent (`sprite: None`) — obstacles and walls don't need
    /// art. A nonzero `collider_half_w`/`collider_half_h` also gives it a
    /// dynamic `rapier2d` box collider (`0.0` spawns a purely visual
    /// entity, no physics body — meaningless combined with `sprite: None`,
    /// since a colorless square has nothing to draw at). Spawning with an
    /// `entity_id` already in use replaces that entity, the same
    /// replace-on-republish semantics `gfx::DrawSprite` batches already use.
    Spawn {
        entity_id: u32,
        x: f32,
        y: f32,
        sprite: Option<Sprite>,
        square_color: (u8, u8, u8, u8),
        collider_half_w: f32,
        collider_half_h: f32,
        body_kind: BodyKind,
    },
    /// Sets a spawned entity's rapier2d linear velocity directly — the
    /// mechanism a caller (an extension reading `input/*`) drives movement
    /// with. A no-op if `entity_id` names an entity with no collider, or
    /// no entity at all.
    SetVelocity { entity_id: u32, vx: f32, vy: f32 },
    /// Removes an entity (and its collider, if any) entirely.
    Despawn { entity_id: u32 },
    /// Overwrites a spawned entity's `SquareColor` in place — the
    /// mechanism a caller uses for a temporary flash (set, wait, set
    /// back), without `game-core` itself needing to know about flash
    /// timing. A no-op if `entity_id` names no entity, or one with no
    /// `SquareColor` (a sprite entity, which has none).
    SetColor {
        entity_id: u32,
        color: (u8, u8, u8, u8),
    },
    /// Globally toggles a yellow unfilled outline over every collider-
    /// bearing entity's actual rapier2d extent (sprite entities, plain
    /// squares, and tilemap colliders alike), drawn on top of each
    /// entity's normal draw each tick — a debug aid for checking a
    /// visible sprite/square lines up with what it actually collides as.
    /// Not addressed by `entity_id`: applies to the whole simulation.
    SetDebugHitboxes { enabled: bool },
}

const TAG_SPAWN: u8 = 0;
const TAG_SET_VELOCITY: u8 = 1;
const TAG_DESPAWN: u8 = 2;
const TAG_SET_COLOR: u8 = 3;
const TAG_SET_DEBUG_HITBOXES: u8 = 4;

impl EntityOp {
    pub(super) fn encode_into(&self, writer: Writer) -> Writer {
        match self {
            EntityOp::Spawn {
                entity_id,
                x,
                y,
                sprite,
                square_color,
                collider_half_w,
                collider_half_h,
                body_kind,
            } => {
                let (r, g, b, a) = *square_color;
                let writer = writer.u8(TAG_SPAWN).u32(*entity_id).f32(*x).f32(*y);
                let writer = match sprite {
                    Some(sprite) => writer
                        .u8(1)
                        .u32(sprite.sprite_id)
                        .u32(sprite.frame_w)
                        .u32(sprite.frame_h)
                        .u32(sprite.frame_count)
                        .f32(sprite.frame_duration),
                    None => writer.u8(0).u32(0).u32(0).u32(0).u32(0).f32(0.0),
                };
                writer
                    .u8(r)
                    .u8(g)
                    .u8(b)
                    .u8(a)
                    .f32(*collider_half_w)
                    .f32(*collider_half_h)
                    .u8(match body_kind {
                        BodyKind::Dynamic => 0,
                        BodyKind::Kinematic => 1,
                        BodyKind::Frictionless => 2,
                    })
            }
            EntityOp::SetVelocity { entity_id, vx, vy } => writer
                .u8(TAG_SET_VELOCITY)
                .u32(*entity_id)
                .f32(*vx)
                .f32(*vy),
            EntityOp::Despawn { entity_id } => writer.u8(TAG_DESPAWN).u32(*entity_id),
            EntityOp::SetColor { entity_id, color } => {
                let (r, g, b, a) = *color;
                writer
                    .u8(TAG_SET_COLOR)
                    .u32(*entity_id)
                    .u8(r)
                    .u8(g)
                    .u8(b)
                    .u8(a)
            }
            EntityOp::SetDebugHitboxes { enabled } => writer
                .u8(TAG_SET_DEBUG_HITBOXES)
                .u8(if *enabled { 1 } else { 0 }),
        }
    }

    pub(super) fn decode(reader: &mut Reader) -> Result<Self, DecodeError> {
        let tag = reader.read_u8()?;
        match tag {
            TAG_SPAWN => {
                let entity_id = reader.read_u32()?;
                let x = reader.read_f32()?;
                let y = reader.read_f32()?;
                let has_sprite = reader.read_u8()? != 0;
                let sprite_id = reader.read_u32()?;
                let frame_w = reader.read_u32()?;
                let frame_h = reader.read_u32()?;
                let frame_count = reader.read_u32()?;
                let frame_duration = reader.read_f32()?;
                let sprite = has_sprite.then_some(Sprite {
                    sprite_id,
                    frame_w,
                    frame_h,
                    frame_count,
                    frame_duration,
                });
                let square_color = (
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                );
                let collider_half_w = reader.read_f32()?;
                let collider_half_h = reader.read_f32()?;
                let body_kind = match reader.read_u8()? {
                    1 => BodyKind::Kinematic,
                    2 => BodyKind::Frictionless,
                    _ => BodyKind::Dynamic,
                };
                Ok(EntityOp::Spawn {
                    entity_id,
                    x,
                    y,
                    sprite,
                    square_color,
                    collider_half_w,
                    collider_half_h,
                    body_kind,
                })
            }
            TAG_SET_VELOCITY => Ok(EntityOp::SetVelocity {
                entity_id: reader.read_u32()?,
                vx: reader.read_f32()?,
                vy: reader.read_f32()?,
            }),
            TAG_DESPAWN => Ok(EntityOp::Despawn {
                entity_id: reader.read_u32()?,
            }),
            TAG_SET_COLOR => {
                let entity_id = reader.read_u32()?;
                let color = (
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                    reader.read_u8()?,
                );
                Ok(EntityOp::SetColor { entity_id, color })
            }
            TAG_SET_DEBUG_HITBOXES => Ok(EntityOp::SetDebugHitboxes {
                enabled: reader.read_u8()? != 0,
            }),
            _ => Err(DecodeError::InvalidTag {
                message: "game-core entity op",
                tag,
            }),
        }
    }
}
