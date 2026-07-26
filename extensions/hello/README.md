# hello

Reference WASM extension exercising the full contract (`wit/core.wit`):
subscribes to `core/tick` in `init`, logs on every `init`, `on-tick`, and
`on-message`, and publishes a `hello/received` envelope for every message
it gets. It also subscribes to the application close request and publishes
`hello/cleanup` from `shutdown`, making orderly cleanup visible in the demo.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`) — cross-platform, so this is the only build
script needed on any OS. It runs `rustup target add wasm32-wasip2` (safe to
repeat) then `cargo build --target wasm32-wasip2 --release`.

Output: `target/wasm32-wasip2/release/hello.wasm` — a WASM component, loaded
directly by `core/host` (no separate componentization step needed).

Building directly with `cargo build` (no `--target`) does not error — it
silently compiles a native `hello.dll`/`.so` instead, since this crate has no
other way to know which platform you meant. Always go through the script.

The same script also builds the `bones` engine and assembles a runnable
`dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and
`dist/extensions/hello.wasm`, ready to run directly with no further setup.
