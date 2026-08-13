# ADR-026: Game core publishes authoritative entity transform snapshots

## Problem

WASM gameplay systems can command entity velocity and observe collision starts, but collision resolution can move an entity away from any position the caller could predict. Combat, pursuit, and other spatial rules need the same authoritative world-space centers that game-core renders.

## Decision

Game-core publishes one typed `game-core/entity-transform` snapshot for every caller-addressable entity on each tick:

- snapshots follow physics and ECS transform synchronization;
- snapshots are ordered by caller-assigned entity id;
- paused ticks repeat the frozen transforms;
- tilemap-internal colliders are excluded because they have no caller id.

## Rationale

An outbound snapshot keeps physics ownership inside game-core while letting independent extensions make accurate spatial decisions. A fixed message is language-neutral, easy to ignore, and does not enlarge the command surface or expose ECS handles.

## Rejected alternatives

- **Let extensions integrate commanded velocity** — collision correction and multi-world synchronization make predicted positions drift from reality.
- **Add synchronous transform queries** — creates per-entity request traffic and couples guest ticks to a native call path.
- **Move combat into game-core** — makes a reusable simulation module own game-specific health, targeting, and reward rules.
