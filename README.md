# bones

A small native engine core — windows, tray icon, input (SDL), logging, and a
message bus — with all product behavior implemented as WASM extensions in
any language. See [docs/architecture.md](docs/architecture.md) for the full
design.

**Status:** kernel (bus, extension host, platform, logging), the renderer
and egui ui modules, audio, the game-core module (ECS, collision, tilemap
loading, sprite-animation timing), hot reload, and custom native-module
injection (`.module(...)`, see `embedding-demo/`) all work today. The web
presentation module and extension queue budgets remain — see
[docs/roadmap.md](docs/roadmap.md).

Use cases:

- A game engine with quick prototyping: implement a game module, ship
  levels as easily reloadable WASM extensions.
- A GUI application: implement a UI module, write the app's logic as
  extensions.

## Quickstart

```sh
pwsh dist.ps1
```

Builds the engine and every extension into `dist/` — run `dist/bones(.exe)`
directly. Or, without a self-contained build:

```sh
cargo run -p app
```

Drop a built `.wasm` extension into `extensions/` next to wherever you run
it (see `extensions/hello/README.md` to build the reference extension).

## Documentation

Start at [docs/index.md](docs/index.md) — map of the architecture,
detailed designs, decisions (ADRs), and worked examples.

## Demos

Game:

![demo](docs/README/demo.gif)
