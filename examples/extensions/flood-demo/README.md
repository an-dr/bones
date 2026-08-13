# flood-demo

Proves the queue half of the extension watchdog (ADR-007). On its first tick, `flood_demo` attempts 64 publishes against the packaged demo's allowance of eight. Excess messages are dropped and counted, the extension is faulted and quarantined, and the bundled `hello` peer keeps receiving ticks.

## Run

```sh
pwsh build.ps1
./dist/bones
```

Requires PowerShell 7+ (`pwsh`). The script builds both WASM components and the native engine, then assembles `dist/` with `bones.toml`, `flood_demo.wasm`, and `hello.wasm`. The log shows `hello` continuing after the engine reports `flood_demo` with its publish drop count. Close the window to stop cleanly.
