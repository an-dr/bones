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

- `game-core/spawn-entity` — spawns one entity, addressable afterward by
  its caller-assigned `entity_id`, with a `Transform` and a
  `SpriteAnimation`. A nonzero `collider_half_w`/`collider_half_h` also
  gives it a dynamic `rapier2d` box collider (`0.0` spawns a purely
  visual entity, no physics body).
- `game-core/set-velocity` — sets a spawned entity's rapier2d linear
  velocity directly, addressed by `entity_id`. The mechanism a caller
  (e.g. an extension reading `input/*` for WASD/gamepad movement) drives
  an entity with; a no-op for an unknown `entity_id` or one with no
  collider.
- Every `core/tick`: sprite-animation timers advance, `rapier2d` steps
  once, and every collider-bearing entity's `Transform` is overwritten
  from its rigid body's post-step position — physics owns position for
  those entities, not the other way around.
- `game-core/load-tilemap` — parses Tiled `.tmx` XML bytes; every rectangle
  object on an object layer named `"Collision"` becomes a static (fixed)
  `rapier2d` collider. Any other layer (tile layers, other object layers)
  is ignored — drawing the tilemap itself is a `gfx/*` concern, out of
  this crate's scope. A map with no `"Collision"` layer loads fine and
  adds no colliders.

No logger, same stance as `core/audio`: registered via the generic
`.module(...)` path, which has no access to `Engine`'s internal logger.
Unlike `audio`, this module also needs to publish (not just receive), so
`Module::init` consumes the `bus` service `Engine::build` provides
unconditionally (design/modules.md) — `init` fails if none is available.

See `extensions/game_core_demo` for a runnable example: a small tilemap
with several static obstacles, a WASD/gamepad-controlled entity, and a
few stationary colliding entities, all animating.
