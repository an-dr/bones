# persistence-demo

Loads its own counter from `persistence` at `init` (a direct `send` — 0 if nothing was ever saved), then increments and saves it roughly once a second. Proves `persistence/*` end to end: an extension's state survives being reloaded, because a real file on disk backs it, not just its in-memory state (which `docs/design/extensions.md` already documents as lost on every reload).

## Build

```sh
cargo xtask example persistence-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/persistence_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/persistence_demo.wasm`, ready to run directly.
