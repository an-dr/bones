# physics

The `PhysicsBackend` trait (ADR-021): a backend-agnostic contract for one
simulated physics world — spawn/remove a body, set/read velocity and
position, step, and query contacts. No simulation lives here; it exists so
`game-core` can drive more than one physics engine without depending on
any of them directly.

`physics-rapier2d` and `physics-retro` are the two implementations today.
`game-core` owns one instance of each and addresses bodies through the
opaque `BodyHandle`/`ColliderHandle` this crate defines — a handle is only
ever meaningful to the `PhysicsBackend` instance that issued it.

Positions, velocities, and extents are `glam::Vec2` — no backend-specific
math type (rapier2d's `nalgebra` vectors, or anything a retro backend might
use internally) crosses this boundary.
