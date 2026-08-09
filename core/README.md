# core

The native engine, one directory per component. Two tiers (ADR-011), and the
tier a crate belongs to decides what may depend on it.

## Kernel — always present

| Crate | Responsibility |
| --- | --- |
| [bus](bus/README.md) | Topics, direct request/reply, delivery semantics, the `Module` contract and typed service registry, queue budgets |
| [wasm-extensions](wasm-extensions/README.md) | Everything about a WASM extension's life: loading, dispatch, watchdog, state events, persistence (ADR-020) |
| [contract](contract/README.md) | Host-side bindings generated from [`wit/`](../wit/README.md) |
| [platform](platform/README.md) | SDL window, tray, input, timing, event pump, headless mode |
| [runner](runner/README.md) | The frame loop and the builder API embedders use |
| [logging](logging/README.md) | Structured sink, per-extension tagging |

## Native modules — optional, consumer-composed

All individually optional; the kernel must build and run with none of them
registered. Embedders add their own the same way (ADR-017).

| Crate | Responsibility |
| --- | --- |
| [renderer](renderer/README.md) | Executes `gfx/*` draw batches, presents |
| [ui](ui/README.md) | egui: widget specs in, draw data and events out |
| [audio](audio/README.md) | `audio/*` music and effects, backed by kira |
| [game-core](game-core/README.md) | ECS, collision, tilemaps, sprite animation |
| [web](web/README.md) | wry panels and the bus/page JSON bridge |

## The one binary

[app](app/README.md) is the shipped engine executable — the default
composition. It has **no access an embedder lacks**: it uses only the public
builder API, which is what [examples/embedding-demo](../examples/embedding-demo/README.md)
exists to prove.

## Rules

- Extensions depend on `contract` only, never on core internals.
- `bus` and `contract` know nothing about presentation — messaging must stay
  usable headless.
- `logging` is a universal leaf: anyone may depend on it, it depends on
  nothing.
- A module never depends on another module's crate; it goes through a service
  in the registry `bus` owns.
- Nothing depends on `app`.

The dependency graph, and what counts as a violation, is in
[docs/structure.md](../docs/structure.md). File-layout conventions —
one type per file, tests out of line, what a crate README should say — are in
[docs/code-style.md](../docs/code-style.md).
