/// A body's collider/visual shape — every `PhysicsBackend` implementation
/// must give each variant a meaning, even a backend (like the retro/arcade
/// one) with no real non-rectangular collision concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    /// An axis-aligned box at the body's half-extents — the default, every
    /// caller's behavior before `Shape` existed.
    #[default]
    Rect,
    /// An isoceles triangle inscribed in the same half-extents box (apex
    /// centered on top, base along the bottom). A backend with no real
    /// triangle collider (the retro/arcade one) may approximate this as
    /// its own `Rect` bounding box instead — see that backend's own docs
    /// for whether it does.
    Triangle,
}

#[cfg(test)]
mod tests;
