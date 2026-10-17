# level-demo

Proves hot reload (design/extensions.md's Reloading state): logs `level_demo {VERSION}: loaded` once on `init`.

## Live reload demo

1. Build and package it, then start the assembled engine:

   ```sh
   cargo xtask example level-demo
   dist/bones
   ```
2. With `bones` running, edit `VERSION` in `src/lib.rs`, then rebuild — this re-copies the changed `.wasm` into `dist/extensions/`, which is where the running engine is watching:

   ```sh
   cargo xtask example level-demo
   ```
3. Watch the log: the next line shows the new version — the running engine picked up the changed `.wasm` file without a restart.

## Build

```sh
cargo xtask example level-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/level_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/level_demo.wasm`, ready to run directly.
