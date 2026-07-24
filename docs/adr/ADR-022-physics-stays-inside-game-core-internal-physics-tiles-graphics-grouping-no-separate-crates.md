# ADR-022: Physics stays inside game-core: internal physics/tiles/graphics grouping, no separate crates

## Problem

ADR-021 split physics out of `game-core` into three crates —
`core/physics` (the `PhysicsBackend` trait), `core/physics-rapier2d`, and
`core/physics-retro` — reasoning that the trait boundary and swappable
backends warranted independent crates on `structure.md`'s "one directory
per component" model. On review, that split surfaces `physics` as a
sibling of `game-core` in the workspace and `structure.md`'s dependency
graph — implying it's a component other native modules might reasonably
depend on independently — when in practice nothing outside `game-core`
uses it, and it exists purely to serve `game-core`'s own internal
ECS/simulation loop. The question is where the trait-based backend
abstraction should live, not whether it should exist at all.

## Decision

Supersedes ADR-021's crate split. `physics-rapier2d`, `physics-retro`,
and the `PhysicsBackend` trait move back into `core/game-core` as internal
submodules; `core/physics` is deleted. `game-core`'s own source is
grouped into three internal areas reflecting its actual concerns:

- `physics/` — the `PhysicsBackend` trait, `Rapier2dBackend`,
  `RetroBackend`, `Collider`, `WorldBody`, `PhysicsWorldKind`, `BodyHandle`/
  `ColliderHandle`/`BodyKind`.
- `graphics/` — `SpriteAnimation`, `SquareColor`, `Transform` (the
  drawable/visual-state side of an entity).
- `tiles/` — tilemap loading (`load_collision_rects`, `CollisionRect`).

`GameCore` itself, and the module's `lib.rs`/`Cargo.toml`, stay at the
crate root. The trait-based backend swap ADR-021 wanted (rapier2d vs.
retro, selectable per entity, ADR-021's multi-world priority/snapping
model) is unchanged in behavior — only its crate boundary moves from
"three sibling crates" to "one crate, three internal modules."

## Rationale

- Nothing outside `game-core` ever depended on `physics`,
  `physics-rapier2d`, or `physics-retro` — the split crates added
  workspace surface area (three more `Cargo.toml`s, READMEs, entries in
  `structure.md`'s dependency graph) without a second consumer to justify
  the boundary.
- `structure.md`'s "one directory per component" rule describes
  first-party *modules and kernel pieces* (renderer, audio, game-core
  itself) — internal implementation structure inside one of those
  components has always been a file-layout concern (`docs/code-style.md`),
  not a new crate. Physics/tiles/graphics submodules inside `game-core`
  fit that existing convention directly.
- The `PhysicsBackend` trait's value — letting `game-core` hold multiple
  interchangeable backend implementations behind one interface — does not
  require a crate boundary to work; a trait with two impls in the same
  crate is exactly as swappable, just without the extra workspace
  ceremony.

## Rejected alternatives

- **Keep ADR-021's three-crate split** — rejected per the problem above:
  no second consumer ever emerged to justify treating physics as an
  independent, embeddable component.
- **Partial fold (keep `core/physics` the trait crate, fold only the two
  backend crates into `game-core`)** — considered, but leaves the same
  "why does this trait need its own crate with one consumer" question
  half-answered; folding all three together is the simpler, fully
  consistent outcome.
