# audio-demo

Loads two synthesized tones (no embedded audio asset — both are generated in Rust at `init`, sine waves over an exact whole number of cycles so a one-shot beep has no click and the looping track has no seam) and loops the lower one as background music. Pressing any key plays the higher one as a one-shot sound effect. Proves `audio/*` end to end: load, play, and looping music through a real `bones-module-audio` module.

## Build

```sh
cargo xtask example audio-demo
```

Run it from the repository root. The component alone builds in this directory with `cargo build --target wasm32-wasip2 --release`; output: `target/wasm32-wasip2/release/audio_demo.wasm`.

The command also builds the `bones` engine and assembles a runnable `dist/` next to this README — `dist/bones(.exe)`, `dist/extensions/ audio_demo.wasm`, and a `dist/bones.toml` with `audio = true` already set (`audio` defaults off — see `crates/bones`'s config), ready to run directly.
