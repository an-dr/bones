# game_core_demo

Loads a Tiled `.tmx` level (an open arena — just its outer boundary
walls on a `"Collision"` object layer, no interior obstacles to path
around) and a sprite, then spawns a WASD/gamepad-controlled sprite
entity, four stationary red obstacle squares, and two blue immovable
squares in `init`. Proves `game-core` end to end: `rapier2d` blocks the
controlled entity against the tilemap's fixed boundary colliders, the
red obstacles' `Dynamic` colliders (pushable), and the blue squares'
`Kinematic` colliders (solid but never pushed); the controlled entity
animates through its sprite's 4 frames only while moving, freezing on
its current frame at rest; everything renders through `gfx/*` — all
driven by `game-core`'s own `core/tick` subscription, not this
extension's.

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

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/game_core_demo.wasm`.

## Run

Enable `game_core` and `audio` in `bones.toml` (see `core/app`'s config),
then drop the built `.wasm` into `extensions/` next to the `bones`
executable. WASD or a connected gamepad's left stick moves the
controlled entity around the open arena into the stationary obstacles,
the blue squares, and the boundary walls at the arena's edge.
