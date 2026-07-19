# game-core

A native 2D game simulation module (ADR-019, roadmap #4): entity/component
store, collision, tilemap loading, and sprite-animation timing, composed
from bought, engine-agnostic crates rather than written from scratch or
taken from adopting an external engine — see ADR-019's crate-sourcing
rationale.

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

- `game-core/spawn-entity` — spawns one entity with a `Transform` and a
  `SpriteAnimation`. A nonzero `collider_half_w`/`collider_half_h` also
  gives it a dynamic `rapier2d` box collider (`0.0` spawns a purely
  visual entity, no physics body).
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
