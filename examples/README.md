# Examples

Working code you can build and run. Each proves one engine capability end to end; none is part of a bones distribution.

Two groups, because there are two ways to use bones:

- [extensions/](extensions/) — WASM components loaded by the shipped engine. This is what most projects write.
- [embedding/](embedding/) — a parent project that links the engine as a library and injects its own native module.

The one extension that *is* shipped lives in [crates/bones-extension-hello](../crates/bones-extension-hello/README.md): the reference extension, and the place to start if you are writing your own.

## Extensions

| Example | Proves |
| --- | --- |
| [sprite-demo](extensions/sprite-demo/README.md) | An extension drives the renderer: loads a sprite, draws it every tick |
| [notes-demo](extensions/notes-demo/README.md) | An extension drives the egui `ui/*` backend (ADR-005) — the worked example in [docs/examples/egui-app.md](../docs/examples/egui-app.md) |
| [dashboard-demo](extensions/dashboard-demo/README.md) + [metrics-demo](extensions/metrics-demo/README.md) | Two extensions exchange pushed updates and direct history requests through a real wry web panel — see [docs/examples/web-app.md](../docs/examples/web-app.md) |
| [keyecho-demo](extensions/keyecho-demo/README.md) | SDL keyboard, mouse, and gamepad input reaches an extension |
| [audio-demo](extensions/audio-demo/README.md) | The audio module plays looping music and one-shot effects |
| [game-core-demo](extensions/game-core-demo/README.md) | The game-core module: a Tiled level, ECS entities, collision, and a controllable sprite |
| [level-demo](extensions/level-demo/README.md) | Hot reload — the Reloading state in [docs/design/extensions.md](../docs/design/extensions.md) |
| [persistence-demo](extensions/persistence-demo/README.md) | Extension state survives a reload because a real file backs it |
| [runaway-demo](extensions/runaway-demo/README.md) | The time half of the watchdog (ADR-007): a hung `on-tick` is trapped and quarantined |
| [flood-demo](extensions/flood-demo/README.md) | The queue half of the watchdog: a publish flood is dropped, counted, and faulted |

## Embedding

| Example | Proves |
| --- | --- |
| [custom-engine](embedding/custom-engine/README.md) | Your own `bones` executable: the shipped stack plus a native module, plus a message vocabulary bones does not own, spoken by a WASM extension on the other side of the sandbox |

It is three crates rather than one, and that is the lesson rather than an accident: the vocabulary has to compile for the native host *and* for `wasm32-wasip2`, so it cannot live in either the binary or the guest.

## Why every name ends in -demo

An extension's bus endpoint name is the stem of its built `.wasm` file, and that namespace is shared with the native modules: `audio`, `game-core`, and `persistence` are already taken. The suffix keeps an example from colliding with the very module it demonstrates. It is not decoration, so do not drop it when adding one.

## Building one

Every example carries its own `build.ps1`, which builds the extension for `wasm32-wasip2`, builds the engine, and assembles a self-contained `dist/` beside it:

```sh
pwsh examples/extensions/sprite-demo/build.ps1
```

Run the `dist/bones(.exe)` it produces. Requires PowerShell 7+ (`pwsh`), which is cross-platform, so this is the only build script needed on any OS.

Two things to know before you copy this pattern:

- **`dashboard-demo` and `metrics-demo` are a bound pair.** `dashboard-demo`'s build script reaches sideways for `metrics-demo` and packages both; building `metrics-demo` alone produces an extension with nothing to talk to.
- **Examples are not in the distribution.** [dist.ps1](../dist.ps1) builds `extensions/` only, so `pwsh dist.ps1` ships the engine and `hello` — a distribution is the engine plus its reference extension, not a demo bundle. Use an example's own `build.ps1` to run it.

## Running the whole test suite

Several of these double as fixtures for the integration tests, which load their built `.wasm` by path. [test.ps1](../test.ps1) builds every fixture it needs and then runs everything:

```sh
pwsh test.ps1
```
