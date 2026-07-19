# game-core

A native 2D game simulation module (ADR-019): entity/component store,
collision, tilemap loading, and sprite-animation timing, composed from
bought, engine-agnostic crates rather than written from scratch or taken
from adopting an external engine — see ADR-019's crate-sourcing rationale.

| Capability | Source |
| ---------- | ------ |
| Entity/component store | `hecs` (chosen over `bevy_ecs`: no window/asset/App-plugin coupling, materially smaller dependency footprint standalone) |
| Physics/collision | `rapier2d` |
| Math (vectors/transforms) | `glam` |
| Tilemap data | `tiled` (parsing only — rendering stays `gfx/*`) |
| Sprite-animation timing | built directly — a frame-index-from-elapsed-time state machine |

Renders by turning simulated state into `gfx/*` draw-command batches each
tick, the same as any other module or extension — this crate has no
rendering authority of its own.

- `game-core/entity-op` — one topic carrying a tagged `EntityOp` (`Spawn`,
  `SetVelocity`, `Despawn`), open/closed: a future operation extends this
  enum, not the topic list, the same pattern `ui::Widget` uses for
  `ui/spec`.
  - `Spawn` — spawns (or, for an `entity_id` already in use, replaces)
    one entity, addressable afterward by that caller-assigned `entity_id`.
    Carries either an animated `sprite` or a plain filled `square_color`
    square (obstacles/walls that don't need art). A nonzero
    `collider_half_w`/`collider_half_h` also gives it a `rapier2d` box
    collider (`0.0` spawns a purely visual entity, no physics body) of
    the given `body_kind`: `Dynamic` (pushed by other bodies, carries
    momentum, the default), `Kinematic` (moves exactly as `SetVelocity`
    commands it and pushes `Dynamic` bodies out of its way, but is never
    itself pushed — the standard "platform/mover" body type), or
    `Frictionless` (a `Dynamic` body under very high linear damping with
    rotation locked: pushed by contact same as `Dynamic`, but carries no
    momentum — it settles to rest almost immediately once nothing is
    pushing it, rather than coasting or drifting).
  - `SetVelocity` — sets a spawned entity's rapier2d linear velocity
    directly, addressed by `entity_id`. The mechanism a caller (e.g. an
    extension reading `input/*` for WASD/gamepad movement) drives an
    entity with; a no-op for an unknown `entity_id` or one with no
    collider.
  - `Despawn` — removes an entity and its collider, if any. A no-op for
    an unknown `entity_id`.
  - `SetColor` — overwrites a spawned entity's `SquareColor` in place —
    the mechanism a caller uses for a temporary flash (set, wait, set
    back) without `game-core` itself needing to know about flash timing.
    A no-op for an unknown `entity_id`, or one with no `SquareColor` (a
    sprite entity has none).
  - `SetDebugHitboxes { enabled }` — not addressed by `entity_id`: a
    global toggle (default off) for a yellow unfilled outline drawn over
    every collider-bearing entity's actual `rapier2d` extent — sprite
    entities, plain squares, and tilemap colliders alike — on top of
    each entity's normal draw. A debug aid for checking a visible
    sprite/square actually lines up with what it collides as.
- Every `core/tick`: `rapier2d` steps once, every collider-bearing
  entity's `Transform` is overwritten from its rigid body's post-step
  position (physics owns position for those entities, not the other way
  around); a sprite entity's animation only advances while its collider's
  linear speed is above a small threshold — an entity with no collider, or
  one at rest, freezes on its current frame instead of animating in
  place. Then `gfx::Clear` + `gfx::SetCamera` + one
  `gfx::DrawSprite`/`gfx::DrawRect` per entity are published. The `Clear`
  matters: without it, the renderer's retained batches never erase the
  previous frame, and the scene visibly smears.
- `game-core/load-tilemap` — parses Tiled `.tmx` XML bytes; every rectangle
  object on an object layer named `"Collision"` becomes a static (fixed)
  collider, drawn as a plain square in a fixed color distinct from
  `EntityOp::Spawn`'s caller-chosen `square_color` — an invisible tilemap
  wall reads as a bug ("why can't I move here?"), not an intentional
  obstacle. Any other layer (tile layers, other object layers) is
  ignored — drawing the tilemap's own tiles is still a `gfx/*` concern,
  out of this crate's scope; only its collision geometry gets a visual.
  A map with no `"Collision"` layer loads fine and adds no colliders.
  Stays its own topic rather than folding into `EntityOp`: a one-shot
  asset load, not a per-entity operation.
- `game-core/collision` — published whenever two `EntityOp::Spawn`-created
  colliders actually touch, carrying both entities' `entity_id`s
  (unordered — which is `entity_id_a` vs. `entity_id_b` depends on
  rapier2d's internal collider ordering, not on which entity moved into
  the other). Fires once per new contact, not once per tick two entities
  stay overlapping; a tilemap collider (no `entity_id` of its own) never
  appears in one. rapier2d's `CollisionEvent::Started` alone isn't proof
  of a real touch — it also fires for colliders merely within its small
  speculative-contact prediction margin, not yet overlapping — so a
  `Started` event is confirmed against the actual contact manifold
  (`Physics::has_real_contact`) before publishing, filtering out that
  false-positive case.

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
