# physics-retro

`physics::PhysicsBackend` implemented with no mass, no impulses, and no
solver (ADR-021): a body moves by `velocity * dt` each step, then
overlapping pushable bodies are separated once along the axis of least
penetration (a minimum-translation-vector push-apart). `Fixed` and
`Kinematic` bodies are never displaced by that separation, matching every
other backend's "platform/mover is never pushed" contract.

No speculative contact margin and no `contact_normals` output — this
backend already resolves overlap by directly moving positions during
`step`, so there is nothing left for a caller to clamp a velocity against
afterward the way `physics-rapier2d` callers do.

Meant for entities that want an immediate stop/start, no-momentum feel
distinct from `physics-rapier2d`'s full rigid-body simulation — see
ADR-021 for how `game-core` picks between the two per entity.
