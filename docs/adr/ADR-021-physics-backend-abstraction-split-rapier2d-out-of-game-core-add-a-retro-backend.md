# ADR-021: Physics backend abstraction: split rapier2d out of game-core, add a retro backend

## Problem

`game-core`'s `Physics` (`core/game-core/src/physics.rs`) *is* rapier2d: `GameCore` reaches directly into `rapier2d::prelude` types (`ColliderHandle`, `RigidBodyHandle`, `Vector<Real>`) in its own `Collider` component and its own logic (ADR-019 already anticipated this seam — "individual crate choices are tactical, swappable behind game-core's own boundary" — but never cut it). There is no seam a second physics engine could plug into today.

The game needs a second simulation model alongside rapier2d: a retro/arcade style (no mass, no inertia, immediate stop/start, tile-snapped movement) for entities where full rigid-body simulation is the wrong feel. Both must be usable in the same scene, and — the harder requirement — the same *entity* must be able to be simulated by more than one backend at once (e.g. present in both a retro world and a rapier2d world), with a configured priority order deciding whose result wins when they disagree.

## Decision

Split physics out of `game-core` into a backend-agnostic crate plus one crate per backend, and change `game-core`'s physics model from "one simulation" to "N named `PhysicsWorld`s in a fixed priority order, an entity may be registered in any subset of them":

- `core/physics` — the `PhysicsBackend` trait and backend-agnostic types (opaque `BodyHandle`/`ColliderHandle`, `BodyKind`, contact-query results). No rapier2d or retro-specific code.
- `core/physics-rapier2d` — today's `Physics` (rapier2d pipeline, current tuning unchanged) reimplemented behind `PhysicsBackend`.
- `core/physics-retro` — a new backend: velocity-integrated AABB movement, no mass/impulses/solver, resolved by axis-separation.
- `core/game-core` — depends on `core/physics` (the trait) and both backend crates; owns one `PhysicsWorld` instance per backend and a fixed priority list (e.g. `[retro, rapier2d]`, first wins). Never depends on `rapier2d` or backend internals directly anymore.

Per-tick multi-world resolution:

1. Every world steps independently and fully — each simulates its own copy of every body registered in it, under that world's own rules. A body that exists only in one world behaves exactly as today, no overhead.
2. For an entity registered in more than one world, the authoritative position after stepping is read from the **highest-priority world it is in** and written to the entity's `Transform` (what is drawn and exposed to callers).
3. Every other world holding that same entity then has its copy's position/velocity **snapped to match** the winner, before the next tick — so worlds never drift apart, and a body that exists only in a losing world (e.g. a rapier2d-only crate pushing against a retro-priority shared entity) collides against where the entity is actually drawn, not a stale phantom.

`EntityOp::Spawn` (`shared/bones-messages/src/game_core/entity_op.rs`) gains a field naming the set of worlds an entity is registered in, instead of assuming rapier2d. `Collider` becomes backend-agnostic, holding one handle pair per world the entity is registered in.

## Rationale

- Two independent, non-overlapping simulation worlds per shared entity (rather than a shared narrow-phase/contact model across backends) keeps rapier2d's own tuned solver and contact behavior (recent commits tightened `contact_natural_frequency`/`contact_damping_ratio` to fix interpenetration) completely untouched — a shared low-level collision system would have meant rewriting or discarding that tuning for uncertain benefit.
- Full independent stepping plus priority-resolved position snapping (rather than one world holding a shared entity fixed/kinematic while the other owns it) lets every world's *other* bodies keep colliding with a physically real, moving copy of the shared entity under that world's own rules — a rapier2d-only body pushed against a retro-priority shared entity still gets a proper rapier2d contact response, not a stale or frozen collider.
- Snapping losing worlds to the winner's position every tick (rather than letting them run fully independently) keeps all worlds spatially consistent with what's drawn — without it, a losing world's copy could visibly drift from the entity's actual on-screen position over many ticks, making its collisions against other same-world bodies look wrong even though the simulation itself is "correct" in isolation.
- A fixed, explicit priority order (not per-pair negotiation or last-write- wins) makes multi-world conflicts deterministic and simple to reason about — the same simplification a single fixed `ENTITY_LAYER` already makes for rendering in this module.

## Rejected alternatives

- **Shared narrow-phase, split integration** (one engine-agnostic collision-detection layer feeding both backends' integrators) — would give true cross-engine momentum transfer, but effectively replaces rapier2d's internal narrow-phase, discarding tuned, working behavior for a capability not actually needed (worlds only need to *agree on a position*, not resolve a shared contact).
- **One-way AABB obstacle export** (each backend treats the other's bodies as static obstacles, no shared entities) — simpler, but doesn't support the actual requirement: the same entity genuinely present and simulated in more than one world at once, with priority deciding the visible result.
- **One backend per `GameCore` instance** (mixing backends means running two `GameCore` modules) — rejected because it can't express a single entity in multiple worlds at all, only multiple entities each in one world.
