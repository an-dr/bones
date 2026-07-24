/// Which physics world(s) (ADR-021) a spawned entity's body is registered
/// in. An entity may be in more than one at once — `game-core` steps each
/// world it's in independently, then resolves the entity's drawn/exposed
/// `Transform` from a fixed priority order (`retro` before `rapier2d`) and
/// snaps every other world's copy to match, so a body genuinely present in
/// two worlds never drifts between what's drawn and what each world's own
/// collisions see.
///
/// Meaningless combined with a zero-size collider (`collider_half_w`/
/// `collider_half_h` both `0.0`): a purely visual entity has no body to
/// register anywhere, regardless of which worlds are set here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsWorlds {
    /// Register in the `rapier2d`-backed world — full rigid-body
    /// simulation. Defaults to `true`: every caller before `PhysicsWorlds`
    /// existed got a rapier2d body, and an all-`false` default would
    /// silently spawn a visual-only entity for every existing caller.
    pub rapier2d: bool,
    /// Register in the retro/arcade-backed world — no mass, no solver,
    /// immediate stop/start.
    pub retro: bool,
}

impl PhysicsWorlds {
    /// The default before `PhysicsWorlds` existed: every collider-bearing
    /// entity got exactly one rapier2d body.
    pub const RAPIER2D: Self = Self {
        rapier2d: true,
        retro: false,
    };

    /// Retro-only, no rapier2d presence at all.
    pub const RETRO: Self = Self {
        rapier2d: false,
        retro: true,
    };

    /// Registered in both worlds at once (ADR-021's actual multi-world
    /// case) — retro wins position priority, rapier2d's copy is snapped to
    /// match every tick.
    pub const BOTH: Self = Self {
        rapier2d: true,
        retro: true,
    };

    pub(super) fn to_bits(self) -> u8 {
        (self.rapier2d as u8) | ((self.retro as u8) << 1)
    }

    pub(super) fn from_bits(bits: u8) -> Self {
        Self {
            rapier2d: bits & 0b01 != 0,
            retro: bits & 0b10 != 0,
        }
    }
}

impl Default for PhysicsWorlds {
    /// Same as `RAPIER2D` — every caller before this type existed got a
    /// rapier2d body, so an omitted/zero-valued field on the wire decodes
    /// to the same behavior instead of silently becoming a visual-only
    /// entity.
    fn default() -> Self {
        Self::RAPIER2D
    }
}

#[cfg(test)]
mod tests;
