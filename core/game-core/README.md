# game-core

A native 2D game simulation module (ADR-019): entity/component store,
collision, tilemap loading, and sprite-animation timing, composed from
bought, engine-agnostic crates rather than written from scratch or taken
from adopting an external engine — see ADR-019's crate-sourcing rationale.

| Capability | Source |
| ---------- | ------ |
| Entity/component store | `hecs` (chosen over `bevy_ecs`: no window/asset/App-plugin coupling, materially smaller dependency footprint standalone) |
| Physics/collision | an internal backend-agnostic `PhysicsBackend` trait (ADR-021, ADR-022), implemented by `physics::Rapier2dBackend` (`rapier2d`) and `physics::RetroBackend` |
| Math (vectors/transforms) | `glam` |
| Tilemap data | `tiled` (collision geometry and `"Ground"` tile-layer parsing; all drawing still goes out through `gfx/*`, same as every other entity) |
| Sprite-animation timing | built directly — a frame-index-from-elapsed-time state machine |

Renders by turning simulated state into `gfx/*` draw-command batches each
tick, the same as any other module or extension — this crate has no
rendering authority of its own.

**Physics is multi-world (ADR-021, ADR-022).** `GameCore` owns one
`physics::Rapier2dBackend` world and one `physics::RetroBackend` world
(both live in this crate's own `physics/` submodule — see
`docs/code-style.md`'s file-layout conventions — not a separate crate),
and `EntityOp::Spawn`'s `worlds` field (`bones_messages::game_core::
PhysicsWorlds`) picks which one (or both) a spawned entity's body
registers in. The two worlds never interact directly — no shared
collision detection between them. An entity registered in both is fully
simulated by each, but only one world's position is authoritative:
`physics::PhysicsWorldKind::PRIORITY` (retro before rapier2d) picks which
world's position/velocity is read into the drawn `Transform` each tick,
and every other world's copy of that entity is then snapped to match, so
no world drifts from what's actually on screen.

`Rapier2dBackend::new` tunes rapier2d's contact solver stiffer than its
defaults (higher `contact_natural_frequency`, lower
`contact_damping_ratio`, more solver/stabilization iterations):
rapier2d's default spring-based contact resolution is compliant enough
that a body driven continuously into an obstacle (a player entity holding
a direction key against a wall, this crate's typical case) visibly
interpenetrates rather than fully separating each step. The tuned values
measurably shrink steady-state penetration under sustained driving force
(roughly 7x smaller in the regression test) without reintroducing
bounce/jitter — but tuning alone doesn't fully eliminate it. The actual
mechanism behind visible overlap is `EntityOp::SetVelocity`
hard-overwriting a body's linear velocity every tick from held input: that
re-drives the body into whatever it's touching faster than the solver's
per-step corrective push can undo, regardless of stiffness.
`GameCore::tick` fixes this directly after every world's `step` by zeroing
the component of each non-`Kinematic` collider-bearing entity's velocity
(in every world it's registered in) that points into an active contact's
normal (`PhysicsBackend::contact_normals`) — only the inward part, so
sliding along a wall's free axis still works. `Kinematic` bodies are
excluded: they move exactly as commanded by design and are never pushed,
so there's nothing to clamp. `RetroBackend` has no contact-normal concept
at all (it resolves overlap by directly moving positions during its own
`step`), so this clamp is a no-op there by construction, not a special
case.

- `game-core/entity-op` — one topic carrying a tagged `EntityOp` (`Spawn`,
  `SetVelocity`, `Despawn`), open/closed: a future operation extends this
  enum, not the topic list, the same pattern `ui::Widget` uses for
  `ui/spec`.
  - `Spawn` — spawns (or, for an `entity_id` already in use, replaces)
    one entity, addressable afterward by that caller-assigned `entity_id`.
    Carries either an animated `sprite` or a plain filled `square_color`
    shape (obstacles/walls that don't need art), whose collider/visual
    `shape` (`bones_messages::game_core::Shape`) is `Rect` (an axis-aligned
    box, the default) or `Triangle` (an isoceles triangle inscribed in the
    same half-extents box) — collided against as a real triangle in the
    rapier2d world; the retro world has no non-rectangular collision
    concept, so it approximates a `Triangle` collider as its own AABB
    bounding box. A nonzero `collider_half_w`/`collider_half_h` also gives
    it a collider (`0.0` spawns a purely visual entity, no physics body)
    of the given `body_kind`: `Dynamic` (pushed by other bodies, carries
    momentum, the default), `Kinematic` (moves exactly as `SetVelocity` commands it and
    pushes `Dynamic` bodies out of its way, but is never itself pushed —
    the standard "platform/mover" body type), or `Frictionless` (a
    `Dynamic` body that carries no momentum — it settles to rest almost
    immediately once nothing is pushing it, rather than coasting or
    drifting). `worlds` (`PhysicsWorlds`, ADR-021) picks which physics
    world(s) the collider registers in — `rapier2d` alone by default
    (every caller's behavior before `PhysicsWorlds` existed), `retro`
    alone, or both at once for an entity that needs to be simulated —
    and collide with same-world-only bodies — in two independent worlds
    simultaneously.
  - `SetVelocity` — sets a spawned entity's linear velocity directly, in
    every physics world it's registered in, addressed by `entity_id`. The
    mechanism a caller (e.g. an extension reading `input/*` for
    WASD/gamepad movement) drives an entity with; a no-op for an unknown
    `entity_id` or one with no collider.
  - `Despawn` — removes an entity and its collider, if any. A no-op for
    an unknown `entity_id`.
  - `SetColor` — overwrites a spawned entity's `SquareColor` in place —
    the mechanism a caller uses for a temporary flash (set, wait, set
    back) without `game-core` itself needing to know about flash timing.
    A no-op for an unknown `entity_id`, or one with no `SquareColor` (a
    sprite entity has none).
  - `SetDebugHitboxes { enabled }` — not addressed by `entity_id`: a
    global toggle (default off) for a yellow unfilled outline drawn over
    every collider-bearing entity's actual physics extent — sprite
    entities, plain squares, and tilemap colliders alike — on top of
    each entity's normal draw. A debug aid for checking a visible
    sprite/square actually lines up with what it collides as.
  - `SetPaused { paused }` — not addressed by `entity_id`: a global
    freeze (default off). While `true`, `core/tick` skips physics
    entirely — neither world steps, nothing settles or keeps drifting
    under residual velocity, and no `game-core/collision` can fire —
    every entity holds exactly its last-unpaused state. `gfx/*` still
    publishes every tick regardless, so the frame stays visible (frozen)
    rather than going stale or blank.
- Every `core/tick`: both physics worlds step once each, in full,
  independently. A collider-bearing entity's `Transform` is then
  overwritten from its primary world's post-step position (retro before
  rapier2d, per `PhysicsWorldKind::PRIORITY`) — physics owns position for
  those entities, not the other way around — and, for an entity
  registered in more than one world, every other world's copy is snapped
  to that same position/velocity before the next tick. A sprite entity's
  animation only advances while its primary world's velocity is above a
  small threshold — an entity with no collider, or one at rest, freezes on
  its current frame instead of animating in place. Ground tiles (see
  `game-core/load-tilemap` below) publish first, on layer `0`; then
  `gfx::Clear` + `gfx::SetCamera` + one `gfx::DrawSprite`/`gfx::DrawRect`
  per entity, all on layer `1`. The `Clear` matters: without it, the
  renderer's retained batches never erase the previous frame, and the
  scene visibly smears. A caller publishing its *own* layer-`0` background
  (not from a `"Ground"` layer — this module's own tile draws already
  cover that case) must republish it every tick if it ever publishes any
  other `gfx/*` content itself: the renderer keeps exactly one retained
  batch per sender, a full replace on every publish rather than a merge,
  so a background published only once from a sender that *also* publishes
  something else every tick gets wiped out by that same sender's next
  publish (`core/renderer`'s own doc comment explains the per-sender
  retained-batch mechanics this follows from).
- `game-core/load-tilemap` — parses Tiled `.tmx` XML bytes (embedded
  tilesets only — an externally-referenced `.tsx` or tileset image isn't
  resolved, `tiles::load_tile_draws`'s own doc comment explains why).
  Every rectangle object on an object layer named `"Collision"` becomes a
  static (fixed) collider, drawn as a plain square in a fixed color
  distinct from `EntityOp::Spawn`'s caller-chosen `square_color` — an
  invisible tilemap wall reads as a bug ("why can't I move here?"), not an
  intentional obstacle. A tile layer named `"Ground"` is parsed once here
  and replayed as `gfx::DrawSprite` every tick after (`tick`'s own
  documentation above), one per non-empty cell, resolved through
  `LoadTilemap::tileset_images` — a `.tmx` tileset with no matching image
  name there is parsed (its collision geometry, if any, still works) but
  never drawn. Any other layer is ignored. A map with no `"Collision"`
  layer, no `"Ground"` layer, or neither, loads fine and simply
  contributes nothing for the part it's missing. Stays its own topic
  rather than folding into `EntityOp`: a one-shot asset load, not a
  per-entity operation.
- `game-core/collision` — published whenever two `EntityOp::Spawn`-created
  colliders in the *same* physics world actually touch (worlds never
  interact, ADR-021 — a rapier2d-only entity and a retro-only entity can
  never produce one), carrying both entities' `entity_id`s (unordered —
  which is `entity_id_a` vs. `entity_id_b` depends on the backend's
  internal collider ordering, not on which entity moved into the other).
  Fires once per new contact, not once per tick two entities stay
  overlapping; a tilemap collider (no `entity_id` of its own) never
  appears in one. rapier2d's `CollisionEvent::Started` alone isn't proof
  of a real touch — it also fires for colliders merely within its small
  speculative-contact prediction margin, not yet overlapping — so a
  `Started` event is confirmed against the actual contact manifold
  (`PhysicsBackend::has_real_contact`) before publishing, filtering out
  that false-positive case. `RetroBackend` has no speculative margin at
  all, so every `Started` it reports is already real.

No logger, same stance as `core/audio`: registered via the generic
`.module(...)` path, which has no access to `Engine`'s internal logger.
Unlike `audio`, this module also needs to publish (not just receive), so
`Module::init` consumes the `bus` service `Engine::build` provides
unconditionally (design/modules.md) — `init` fails if none is available.

See `extensions/game_core_demo` for a runnable example: a tilemap, a
WASD/gamepad-controlled sprite entity, red `Dynamic` obstacles, and blue
`Frictionless` squares, all colliding — plus `core/audio` footstep and
hit-flash sound driven entirely from the demo's own `game-core/collision`
handling, not from `game-core` itself.

**Considered, not adopted:** `rapier2d::control::KinematicCharacterController`
— a purpose-built sweep-and-slide controller that guarantees zero
interpenetration by construction (computing allowed movement directly
against swept geometry, rather than a force/velocity-based body fighting
a compliant contact solver). The velocity-clamp approach above already
fixes the reported overlap without an architecture change, so this
wasn't pursued. Worth revisiting if this crate ever needs
guaranteed-no-overlap collision closer to a from-scratch collision model
(the sourcing motivation behind this being a candidate: a prior
from-scratch game project — never fully finished — avoided this class of
bug entirely by construction, not by tuning a bought physics engine).
