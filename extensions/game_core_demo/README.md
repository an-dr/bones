# game_core_demo

Loads a Tiled `.tmx` level (outer walls plus a cross of interior
obstacles, all on a `"Collision"` object layer) and a sprite, then spawns
a WASD/gamepad-controlled entity and four stationary obstacle entities in
`init`. Proves `game-core` end to end: `rapier2d` blocks the controlled
entity against both the tilemap's fixed colliders and the other entities'
dynamic colliders, everything animating through its sprite's 4 frames,
rendered through `gfx/*` — all driven by `game-core`'s own `core/tick`
subscription, not this extension's.

Movement itself is this extension's own logic: it tracks held WASD keys
and the gamepad left stick via `input/*`, and every tick publishes
`game-core/set-velocity` for the entity it spawned with
`entity_id: 1` — `game-core` has no input awareness of its own.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/game_core_demo.wasm`.

## Run

Enable `game_core` in `bones.toml` (see `core/app`'s config), then drop
the built `.wasm` into `extensions/` next to the `bones` executable. WASD
or a connected gamepad's left stick moves the controlled entity into the
tilemap walls and the stationary obstacles.
