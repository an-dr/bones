# Game core

Detailed design of the 2D simulation module: what it owns, how a caller drives it, and what it publishes back. The module's place among the others is in [architecture/overview.md](../architecture/overview.md).

Decisions: [ADR-019](../adr/ADR-019-2d-game-core-module-native-bought-dependencies.md) (native module, bought dependencies), [ADR-022](../adr/ADR-022-physics-stays-inside-game-core-internal-physics-tiles-graphics-grouping-no-separate-crates.md) (physics stays internal), [ADR-023](../adr/ADR-023-evolve-game-core-entity-operations-additively.md) (operations evolve additively), [ADR-026](../adr/ADR-026-game-core-publishes-authoritative-entity-transform-snapshots.md) (authoritative transform snapshots).

## What it is for

A caller — usually a WASM extension holding the game's rules — describes a world in terms of entities, and the module simulates it. The division is deliberate: the module owns *physics and time*, the caller owns *meaning*. Nothing here knows what a player is, what a level is, or when a game ends.

It has no rendering authority. Simulated state becomes `gfx/*` draw commands each tick, exactly as any other module or extension would publish them ([ADR-019](../adr/ADR-019-2d-game-core-module-native-bought-dependencies.md)). The renderer cannot tell the difference.

## Driving it

One topic carries every entity operation, as a tagged union rather than a topic per verb. A new operation extends the union; the topic list does not grow ([ADR-023](../adr/ADR-023-evolve-game-core-entity-operations-additively.md)).

| Concern | Operations |
| --- | --- |
| Existence | `Spawn`, `Despawn`, `Reset` |
| Motion | `SetVelocity` |
| Appearance | `SetColor`, `SetSprite`, `SetSpriteTint` |
| Camera | `SetCameraFollow`, `SetCameraSmoothing` |
| Global state | `SetPaused`, `SetDebugHitboxes` |

Every entity is addressed by a **caller-assigned id**. The module never invents one, so a caller can refer to an entity it spawned without waiting for a reply — which is what keeps the interface one-way and cheap. An operation naming an unknown id is a no-op rather than an error: a caller that despawns an entity twice, or addresses one the simulation already removed, is not doing anything wrong.

Tilemaps load on their own topic, not as an entity operation — a one-shot asset load rather than a per-entity command. Collision geometry and ground tiles are read from named layers; other layers are ignored, and a map missing either layer loads and contributes nothing for the part it lacks.

## Physics is plural

Entities may be simulated by more than one physics backend at once, and a spawn chooses which ([ADR-021](../adr/ADR-021-physics-backend-abstraction-split-rapier2d-out-of-game-core-add-a-retro-backend.md), superseded on crate structure by [ADR-022](../adr/ADR-022-physics-stays-inside-game-core-internal-physics-tiles-graphics-grouping-no-separate-crates.md) but not on this behaviour). Two are shipped: a full rigid-body simulation, and a retro backend that resolves overlap by moving positions directly, the way pre-physics-engine games did.

The worlds never interact. Two entities in different worlds cannot collide, and no collision is reported between them — this is the property that makes a second world useful rather than confusing.

An entity registered in several worlds is simulated by each, but only one is authoritative for its position. A fixed priority picks which, and every other world's copy is snapped to match after each step, so no world drifts from what is drawn. Bodies come in four kinds — pushed and momentum-carrying, commanded and never pushed, immovable, and pushed but momentum-free — which is the vocabulary a caller uses instead of tuning simulation parameters.

## What it publishes

Three things, every tick:

- **Transform snapshots** — one per caller-addressable entity, ordered by entity id, following physics and ECS synchronization so a subscriber sees settled positions rather than mid-step ones. Tilemap-internal colliders are excluded: they have no caller id, so a caller could not act on them. A paused tick repeats the frozen transforms rather than publishing nothing ([ADR-026](../adr/ADR-026-game-core-publishes-authoritative-entity-transform-snapshots.md)).
- **Collisions** — when two caller-spawned colliders in the *same* world begin touching. Once per new contact, not once per tick of continued overlap, and never for a tilemap collider. The pair is unordered: which entity appears first reflects internal collider ordering, not which one moved.
- **Draw commands** — ground tiles beneath, entities above, with the frame cleared first. The clear matters because the renderer retains each sender's last batch: without it the previous frame would never be erased.

## Pausing

Paused is a global freeze, not a slowed tick. No world steps, nothing settles or drifts under residual velocity, and no collision can fire — every entity holds exactly its last-unpaused state. Draw commands and transform snapshots still publish, so the frame stays visible and observers keep seeing the frozen truth rather than stale or absent data.

## Where the detail lives

Message field layouts are specified in [wit/wire-format.md](../../wit/wire-format.md), for every language rather than only Rust. Implementation notes — how the physics backends are tuned, how contacts are filtered, why the internal grouping is what it is — belong beside the code in [bones-module-game-core](../../crates/bones-engine/bones-module-game-core/README.md) and its own doc comments.
