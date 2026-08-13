# ADR-023: Evolve game-core entity operations additively

## Problem

`game-core`'s original `Sprite` is encoded inside `EntityOp::Spawn`. Adding atlas layout, draw scaling, mirroring, or loop control directly to that record would shift every following byte and make existing extension components unreadable. Camera follow has the same constraint: smoothing must not change the established `SetCameraFollow` payload.

## Decision

Keep existing entity-operation tags and payloads byte-for-byte stable. Add new operation tags for richer, optional behavior:

- `SetSprite` applies a `SpritePresentation` to an existing entity in place.
- `SetCameraSmoothing` configures follow responsiveness independently.

New operations may extend the enum, but shipped operation layouts do not grow or change meaning.

## Rationale

Additive operations preserve old WASM compatibility and keep simulation state stable while presentation changes. They also let simple callers continue using the compact original spawn contract.

## Rejected alternatives

- **Extend `Sprite` and `SetCameraFollow` in place** — breaks decoding of components compiled against the existing payloads.
- **Introduce generic component patches** — creates a much larger public ECS surface for two focused behaviors.
- **Despawn and respawn for visual changes** — discards live transform and physics state.
