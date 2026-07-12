# keyecho

Subscribes to `input/key-down` and logs every keypress — proves platform's
SDL input reaches an extension end to end.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/keyecho.wasm`.
