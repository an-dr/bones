# ADR-019: 2D game-core module: native, bought dependencies

## Problem

bones' README already names "a game engine with quick prototyping" as a primary use case, and ADR-011's module/extension rule of thumb ("hot infrastructure native, hot content WASM") already anticipates "an embedder's game core" as a native module. But building an actual 2D game — the working example is an RPG — still needs simulation-level capability bones has none of: an entity/component store, collision, tilemap data, and sprite-animation timing. The open question is not *whether* bones can host a game (ADR-011 already says yes), but where this specific capability comes from and how it is composed.

## Decision

Build a native `game-core` module, consuming ADR-018's camera, persistence, and audio primitives, composed from bought pure-logic crates rather than written from scratch or taken from adopting an external engine:

| Capability | Source |
| --- | --- |
| Entity/component store | `bevy_ecs` (or `hecs` if its footprint proves lighter) |
| Physics/collision | `rapier2d` |
| Math (vectors/transforms) | `glam` |
| Tilemap data | `tiled` (parsing only — rendering stays `gfx/*`) |
| Sprite-animation timing | built directly — a frame-index-from-elapsed-time state machine, no crate warranted |

`game-core` renders by turning simulated state into `gfx/*` draw-command batches, same as any other module or extension — it has no rendering authority of its own.

## Rationale

- ADR-011 already settled the module-vs-extension split; this ADR only fills the "embedder's game core" slot that decision left open, so it inherits that module's trust tier and bus-endpoint symmetry rather than reopening them.
- Every crate picked here is engine-agnostic (no window/asset/App-plugin coupling) — none compete with `renderer` for the window surface or the `window-surface` service, which stays single-consumer per [design/modules.md](../design/modules.md).
- The individual crate choices are tactical (swappable behind `game-core`'s own boundary) — this ADR records the *sourcing strategy* (build the module, buy pure-logic dependencies, reject an external engine), not each library pick as its own irreversible decision.

## Rejected alternatives

- **Embed bones inside an external engine (Bevy, Macroquad)** — inverts ADR-002 (engine-owned rendering) and ADR-011 (kernel + native modules + hot-reloadable WASM extensions): bones would become a plugin inside another engine's scheduler and window ownership instead of the host, discarding the hot-reload/extension model that is the reason bones exists.
- **Write ECS/physics/tilemap parsing from scratch** — no architectural reason to; these are commodity, engine-agnostic, well-tested libraries with no coupling cost here, and hand-writing them is pure effort with no offsetting benefit.
- **Take `bevy_sprite`/`bevy_render`/`bevy_asset` piecemeal from Bevy for rendering or assets** — these assume they own presentation/App-plugin wiring; using them would fight `gfx/*`'s engine-owned rendering (ADR-002) rather than feed it.
