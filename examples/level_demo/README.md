# level_demo

Proves hot reload (design/extensions.md's Reloading state): logs `level_demo {VERSION}: loaded` once on `init`.

## Live reload demo

1. Build and package it, then start the assembled engine:

   ```sh
   pwsh build.ps1
   dist/bones
   ```
2. With `bones` running, edit `VERSION` in `src/lib.rs`, then rebuild — this re-copies the changed `.wasm` into `dist/extensions/`, which is where the running engine is watching:

   ```sh
   pwsh build.ps1
   ```
3. Watch the log: the next line shows the new version — the running engine picked up the changed `.wasm` file without a restart.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output: `target/wasm32-wasip2/release/level_demo.wasm`.

The same script also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/level_demo.wasm`, ready to run directly.
