# game-core

A native 2D game simulation module ([ADR-019](../../../docs/adr/ADR-019-2d-game-core-module-native-bought-dependencies.md)): entity/component store, physics and collision, tilemap loading, and sprite-animation timing. It renders by publishing `gfx/*` draw commands like anything else — it has no rendering authority of its own.

- What it simulates, how a caller drives it, and what it publishes back: [docs/design/game-core.md](../../../docs/design/game-core.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

Built from bought, engine-agnostic crates rather than from scratch — `hecs`, `rapier2d`, `glam`, `tiled` — for the reasons in ADR-019. Physics, graphics, and tiles are internal submodules rather than separate crates ([ADR-022](../../../docs/adr/ADR-022-physics-stays-inside-game-core-internal-physics-tiles-graphics-grouping-no-separate-crates.md)); their implementation notes, including how the rapier2d contact solver is tuned and why velocities are clamped against contact normals, are in the source doc comments beside the code that does it.

`examples/extensions/game-core-demo` is a runnable example: a tilemap, a gamepad-controlled sprite, colliding obstacles, and sound driven from the demo's own collision handling.
