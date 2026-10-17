# notes-demo

The worked example from [docs/examples/egui-app.md](../../../docs/examples/egui-app.md): a text field, an *Add* button, and the growing list of saved notes, all driven through the `ui/*` backend (ADR-005). Proves an extension can drive the ui module end to end.

## Build

```sh
cargo xtask example notes-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/notes_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/notes_demo.wasm`, ready to run directly.
