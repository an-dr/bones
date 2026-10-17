# runaway-demo

Proves the watchdog (ADR-007): its first `on-tick` never returns. The host's per-call time budget traps it, the extension is faulted and quarantined (dropped, unregistered), and the engine — and every other loaded extension — keeps running.

## Build

```sh
cargo xtask example runaway-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/runaway_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/runaway_demo.wasm`, ready to run directly.
