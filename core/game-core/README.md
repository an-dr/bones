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

No logger, same stance as `core/audio`: registered via the generic
`.module(...)` path, which has no access to `Engine`'s internal logger.
