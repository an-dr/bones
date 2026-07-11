# hello

Reference WASM extension exercising the full contract (`wit/core.wit`):
subscribes to `core/tick` in `init`, then logs on every `init`, `on-tick`,
and `on-message`.

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
