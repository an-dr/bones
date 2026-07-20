# keyecho

Subscribes to `input/key-down`, `input/mouse-down`, `input/mouse-up`,
`input/mouse-wheel`, `input/gamepad-connected`, `input/gamepad-disconnected`,
`input/gamepad-button-down`, and `input/gamepad-button-up`, logging each —
proves platform's SDL input reaches an extension end to end. Not
`input/mouse-move` or `input/gamepad-axis` — both fire continuously (every
pixel of cursor travel / every tilt of a stick) and would flood the log.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/keyecho.wasm`.

The same script also builds the `bones` engine and assembles a runnable
`dist/` next to this README — `dist/bones(.exe)` with `dist/extensions/
keyecho.wasm` already in place, ready to run directly.
