# bones

A small native engine core — windows, tray icon, input (SDL), audio, logging,
and a message bus — with all product behavior implemented as WASM extensions
in any language.

Extensions talk to the core and to each other over one message bus. They never
touch the OS and never render directly: they publish draw commands, widget
specs, or web-panel JSON, and the engine presents. That boundary is what makes
an extension hot-reloadable, sandboxed, and language-agnostic.

Two ways to use it:

- **Write extensions only.** Use the shipped engine binary as-is and put your
  game, tool, or GUI in WASM components. Most projects want this.
- **Embed the engine.** Depend on the crates, own the composition root, and
  inject your own native modules with `.module(...)`. For projects that need
  native capabilities the core does not ship.

**Status:** the kernel, renderer, egui UI, audio, game-core, optional wry web
panels, hot reload, orderly shutdown, extension flow-control budgets, and
custom native-module injection all work today. Interfaces are still moving;
this has not had a tagged release yet.

## Quickstart

Requires a Rust toolchain, PowerShell 7+ (`pwsh`, cross-platform), and a C
compiler with CMake — `core/platform` builds SDL3 from source.

```sh
pwsh dist.ps1
```

Builds the engine and the reference extension into `dist/` — run
`dist/bones(.exe)` directly. Or, without a self-contained build:

```sh
cargo run -p app
```

To run the tests, including the integration tests that need built extensions:

```sh
pwsh test.ps1
```

## Your first extension

Start at [extensions/hello](extensions/hello/README.md) — the reference
extension, and the only one a distribution ships. It exercises the whole
contract in [wit/core.wit](wit/core.wit): subscribing in `init`, handling
`on-tick` and `on-message`, publishing, and cleaning up in `shutdown`.

Drop any built `.wasm` into `extensions/` next to wherever you run the engine
and it loads on start.

For something richer, [examples/](examples/README.md) has eleven runnable
examples — sprites, egui widgets, web panels, gamepad input, tilemaps, hot
reload, persistence, and the watchdog — each proving one capability end to
end, each with its own `build.ps1`.

## Projects using bones

| Project | What it is |
| --- | --- |
| [commits](https://github.com/an-dr/commits) | A desktop Git client. Embeds bones and runs its Git Graph view in a wry web panel |
| [artificial-will-game-v2](https://github.com/an-dr/artificial-will-game-v2) | A game about a robot named Will. Embeds bones and ships its gameplay as extensions |
| [copper](https://github.com/an-dr-vibe/an-dr-copper) | A manifest-first automation host for AI-generated extensions. Embeds bones for its settings UI and tray |

## Documentation

Start at [docs/index.md](docs/index.md) — map of the architecture, detailed
designs, decisions (ADRs), and worked examples. The short version:

- [docs/architecture.md](docs/architecture.md) — components, message flows,
  lifecycles.
- [docs/structure.md](docs/structure.md) — what lives where and what may
  depend on what.
- [docs/adr/](docs/adr/) — why the design is the way it is.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build prerequisites, how to run the
tests, and the commit format.

## AI agents

The base agent policy — flows, roles, and skills — lives in
[an-dr/agents](https://github.com/an-dr/agents). Install it globally for your
AI tools; [AGENTS.md](AGENTS.md) holds the repo-specific rules that extend it.

## License

MIT — see [LICENSE](LICENSE).
