# game_core_demo

Loads a Tiled `.tmx` level (an open arena — just its outer boundary
walls on a `"Collision"` object layer, no interior obstacles to path
around) and a sprite, then spawns a WASD/gamepad-controlled sprite
entity, four stationary red obstacle squares, and two blue squares with
no inertia in `init`. Proves `game-core` end to end, including its
multi-world physics (ADR-021, ADR-022): the red obstacles are
rapier2d-only (`PhysicsWorlds::RAPIER2D`, `Dynamic` — pushable, carries
momentum), the blue squares are retro-only (`PhysicsWorlds::RETRO`,
`Frictionless` — pushable too, but with no momentum, stopping the instant
nothing is pushing them rather than coasting), and the controlled entity
is registered in **both** worlds at once (`PhysicsWorlds::BOTH`) — its
drawn position tracks the no-mass, no-solver retro world (higher
priority), while its rapier2d copy is still fully simulated in parallel
and kept snapped to match, so it still blocks against the tilemap's fixed
boundary colliders and gets pushed by/pushes the red obstacles correctly.
The controlled entity animates through its sprite's 4 frames only while
moving, freezing on its current frame at rest; everything renders through
`gfx/*` — all driven by `game-core`'s own `core/tick` subscription, not
this extension's.

Movement itself is this extension's own logic: it tracks held WASD keys
and the gamepad left stick via `input/*`, and every tick publishes a
`game-core/entity-op` `SetVelocity` for the entity it spawned with
`entity_id: 1` — `game-core` has no input awareness of its own. It also
plays a short synthesized footstep tone (via `core/audio`) on a fixed
interval while that velocity is nonzero, and subscribes to
`game-core/collision`: when two of the red obstacles touch, both flash
white for 0.3s (an `EntityOp::SetColor` set then reverted on a timer
this extension tracks itself, not `game-core`) and a distinct hit tone
plays. Pushing a red obstacle into another demonstrates it; the blue
squares never trigger a flash — only obstacle-on-obstacle contact does.
The H key toggles `EntityOp::SetDebugHitboxes`: a yellow unfilled
outline over every collider-bearing entity's actual `rapier2d` extent,
drawn by `game-core` itself — useful for checking a sprite's visible
frame or a square's fill actually lines up with what it collides as.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/game_core_demo.wasm`.

The same script also builds the `bones` engine and assembles a runnable
`dist/` next to this README — `dist/bones(.exe)`, `dist/extensions/
game_core_demo.wasm`, and a `dist/bones.toml` with `game_core = true` and
`audio = true` already set (both default off — see `core/app`'s config),
ready to run directly.

## Run

```sh
dist/bones
```

WASD or a connected gamepad's left stick moves the controlled entity
around the open arena into the stationary obstacles, the blue squares
(which it can push, unlike the red obstacles they don't drift once
released), and the boundary walls at the arena's edge. Press H to toggle
yellow hitbox outlines on every object.
