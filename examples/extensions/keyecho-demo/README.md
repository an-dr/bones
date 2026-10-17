# keyecho-demo

Subscribes to `input/key-down`, `input/mouse-down`, `input/mouse-up`, `input/mouse-wheel`, `input/gamepad-connected`, `input/gamepad-disconnected`, `input/gamepad-button-down`, and `input/gamepad-button-up`, logging each — proves platform's SDL input reaches an extension end to end. Not `input/mouse-move` or `input/gamepad-axis` — both fire continuously (every pixel of cursor travel / every tilt of a stick) and would flood the log.

## Build

```sh
cargo xtask example keyecho-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/keyecho_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/keyecho_demo.wasm`, ready to run directly.
