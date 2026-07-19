# game_core_demo

Loads a small Tiled `.tmx` level (a `"Collision"` object layer with one
floor rect) and a sprite, then spawns two overlapping colliding entities
in `init`. Proves `game-core` end to end: `rapier2d` separates the
entities from each other and rests them on the tilemap's floor collider,
each animating through its sprite's 4 frames, rendered through `gfx/*` —
all driven by `game-core`'s own `core/tick` subscription, not this
extension's.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/game_core_demo.wasm`.

## Run

Enable `game-core` in `bones.toml` (see `core/app`'s config), then drop
the built `.wasm` into `extensions/` next to the `bones` executable.
