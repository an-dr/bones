# physics-rapier2d

`physics::PhysicsBackend` implemented on top of `rapier2d`'s pipeline —
full rigid-body simulation: mass, impulses, a real contact solver. This is
`game-core`'s original physics engine (ADR-019) behind the backend
abstraction cut in ADR-021, tuning unchanged (see `Rapier2dBackend::new`
for the contact stiffness constants and why they differ from rapier2d's
own defaults).

Own opaque `physics::BodyHandle`/`physics::ColliderHandle` values are
minted per `spawn_body` call and mapped internally to rapier2d's own
`RigidBodyHandle`/`ColliderHandle` — nothing rapier2d-specific crosses the
`PhysicsBackend` boundary.
