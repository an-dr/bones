# runaway_demo

Proves the watchdog (ADR-007): its first `on-tick` never returns. The
host's per-call time budget traps it, the extension is faulted and
quarantined (dropped, unregistered), and the engine — and every other
loaded extension — keeps running.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/runaway_demo.wasm`.

The same script also builds the `bones` engine and assembles a runnable
`dist/` next to this README — `dist/bones(.exe)` with `dist/extensions/
runaway_demo.wasm` already in place, ready to run directly.
