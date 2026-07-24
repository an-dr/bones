/// How a body participates in its world's simulation — every
/// `PhysicsBackend` implementation must give each variant a meaning, even
/// a backend (like a retro/arcade one) with no real mass or solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    /// Pushed by other bodies (and gravity, if the backend has any),
    /// participates fully in collision response — the default.
    #[default]
    Dynamic,
    /// Moves exactly as commanded and pushes `Dynamic` bodies out of its
    /// way on contact, but is never itself pushed — the standard
    /// "platform/mover" body type (`kinematic` in Unity, Godot, Box2D,
    /// and rapier2d alike).
    Kinematic,
    /// Never moves and is never pushed — a fixed obstacle.
    Fixed,
    /// A `Dynamic` body with no carried momentum: it moves when pushed,
    /// but velocity decays to rest almost immediately once nothing is
    /// pushing it, instead of coasting or drifting. rapier2d's backend
    /// realizes this via high linear damping and locked rotation; a
    /// retro backend with no damping concept can realize it by simply
    /// zeroing velocity once nothing commands it.
    Frictionless,
}

#[cfg(test)]
mod tests;
