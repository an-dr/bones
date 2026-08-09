# Examples

Working code you can build and run, one directory per example. Each proves
one engine capability end to end — none is part of a bones distribution.

The one extension that *is* shipped lives in
[extensions/hello](../extensions/hello/README.md): the reference extension,
and the place to start if you are writing your own.

## What each example proves

| Example | Proves |
| --- | --- |
| [sprite_demo](sprite_demo/README.md) | An extension drives the renderer: loads a sprite, draws it every tick |
| [notes](notes/README.md) | An extension drives the egui `ui/*` backend (ADR-005) — the worked example in [docs/examples/egui-app.md](../docs/examples/egui-app.md) |
| [dashboard](dashboard/README.md) + [metrics](metrics/README.md) | Two extensions exchange pushed updates and direct history requests through a real wry web panel — see [docs/examples/web-app.md](../docs/examples/web-app.md) |
| [keyecho](keyecho/README.md) | SDL keyboard, mouse, and gamepad input reaches an extension |
| [audio_demo](audio_demo/README.md) | The audio module plays looping music and one-shot effects |
| [game_core_demo](game_core_demo/README.md) | The game-core module: a Tiled level, ECS entities, collision, and a controllable sprite |
| [level_demo](level_demo/README.md) | Hot reload — the Reloading state in [docs/design/extensions.md](../docs/design/extensions.md) |
| [persistence_demo](persistence_demo/README.md) | Extension state survives a reload because a real file backs it |
| [runaway_demo](runaway_demo/README.md) | The time half of the watchdog (ADR-007): a hung `on-tick` is trapped and quarantined |
| [flood_demo](flood_demo/README.md) | The queue half of the watchdog: a publish flood is dropped, counted, and faulted |
| [embedding-demo](embedding-demo/README.md) | Embedding bones as a library and injecting your own native module |

## Building one

Every example carries its own `build.ps1`, which builds the extension for
`wasm32-wasip2`, builds the engine, and assembles a self-contained `dist/`
beside it:

```sh
pwsh examples/sprite_demo/build.ps1
```

Run the `dist/bones(.exe)` it produces. Requires PowerShell 7+ (`pwsh`),
which is cross-platform, so this is the only build script needed on any OS.

Two things to know before you copy this pattern:

- **`dashboard` and `metrics` are a bound pair.** `dashboard`'s build script
  reaches sideways for `metrics` and packages both; building `metrics` alone
  produces an extension with nothing to talk to.
- **Examples are not in the distribution.** [dist.ps1](../dist.ps1) builds
  `extensions/` only, so `pwsh dist.ps1` ships the engine and `hello` — a
  distribution is the engine plus its reference extension, not a demo
  bundle. Use an example's own `build.ps1` to run it.

## Running the whole test suite

Several of these double as fixtures for the integration tests, which load
their built `.wasm` by path. [test.ps1](../test.ps1) builds every fixture it
needs and then runs everything:

```sh
pwsh test.ps1
```
