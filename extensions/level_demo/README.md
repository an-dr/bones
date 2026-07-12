# level_demo

Proves hot reload (design/extensions.md's Reloading state): logs
`level_demo {VERSION}: loaded` once on `init`.

## Live reload demo

1. Build it and drop it next to a running `bones` (or into `extensions/`
   before starting one — see `core/app`'s README):
   ```sh
   pwsh build.ps1
   ```
2. With `bones` running, edit `VERSION` in `src/lib.rs`, then rebuild:
   ```sh
   pwsh build.ps1
   ```
3. Watch the log: the next line shows the new version — the running engine
   picked up the changed `.wasm` file without a restart.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/level_demo.wasm`.
