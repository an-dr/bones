# sprite-demo

Loads `robot_william.png` (a 256x64 strip of four 64x64 frames — only the first is used so far) once in `init`, then every tick clears the screen and draws it at a fixed position. Proves an extension can drive the renderer end to end.

## Build

```sh
cargo xtask example sprite-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/sprite_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/sprite_demo.wasm`, ready to run directly.
