# flood-demo

Proves the queue half of the extension watchdog (ADR-007). On its first tick, `flood_demo` attempts 64 publishes against the packaged demo's allowance of eight. Excess messages are dropped and counted, the extension is faulted and quarantined, and the bundled `hello` peer keeps receiving ticks.

## Run

```sh
cargo xtask example flood-demo
./dist/bones
```

Run it from the repository root. It builds both WASM components and the native engine, then assembles `dist/` with `bones.toml`, `flood_demo.wasm`, and `hello_demo.wasm`. The log shows `hello_demo` continuing after the engine reports `flood_demo` with its publish drop count. Close the window to stop cleanly.
