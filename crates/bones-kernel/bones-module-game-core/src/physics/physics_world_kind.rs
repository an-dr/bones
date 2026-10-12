/// Identifies one of `GameCore`'s physics worlds (ADR-021) — the
/// `game-core`-internal counterpart to `bones_messages::game_core::
/// PhysicsWorlds`' wire bitmask, used as a map key and for the fixed
/// priority order `GameCore` resolves a multi-world entity's position
/// from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicsWorldKind {
    Rapier2d,
    Retro,
}

impl PhysicsWorldKind {
    /// Priority order, highest first: the world an entity registered in
    /// more than one of these reads its authoritative `Transform` from,
    /// with every lower-priority world's copy snapped to match afterward
    /// (ADR-021). `Retro` outranks `Rapier2d` — the ADR's own example
    /// order.
    pub const PRIORITY: [PhysicsWorldKind; 2] =
        [PhysicsWorldKind::Retro, PhysicsWorldKind::Rapier2d];
}

#[cfg(test)]
mod tests;
