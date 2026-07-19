# audio_demo

Loads two synthesized tones (no embedded audio asset — both are generated
in Rust at `init`, sine waves over an exact whole number of cycles so a
one-shot beep has no click and the looping track has no seam) and loops
the lower one as background music. Pressing any key plays the higher one
as a one-shot sound effect. Proves `audio/*` end to end: load, play,
and looping music through a real `core/audio` module.

## Build

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`). Output:
`target/wasm32-wasip2/release/audio_demo.wasm`.
