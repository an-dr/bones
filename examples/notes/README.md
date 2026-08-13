# notes

The worked example from [docs/examples/egui-app.md](../../docs/examples/egui-app.md): a text field, an *Add* button, and the growing list of saved notes, all driven through the `ui/*` backend (ADR-005). Proves an extension can drive the ui module end to end.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output: `target/wasm32-wasip2/release/notes.wasm`.

The same script also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/notes.wasm`, ready to run directly.
