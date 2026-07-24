# sprite_demo

Loads `robot_william.png` (a 256x64 strip of four 64x64 frames — only the
first is used so far) once in `init`, then every tick clears the screen and
draws it at a fixed position. Proves an extension can drive the renderer
end to end.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/sprite_demo.wasm`.

The same script also builds the `bones` engine and assembles a runnable
`dist/` next to this README — `dist/bones(.exe)` with `dist/extensions/
sprite_demo.wasm` already in place, ready to run directly.
