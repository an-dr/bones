# Native modules and the composition root

Detailed design of the module system. Decisions:
[ADR-011](../adr/ADR-011-native-core-modules.md) (kernel + consumer-composed
modules), [ADR-017](../adr/ADR-017-native-module-trait-and-typed-service-registry.md)
(the concrete `Module` trait and service registry). `renderer`, `audio`, and
`web` implement this contract for real; `ui` remains direct-wired (see
structure.md). `wasm-extensions::persistence` also implements it (reusing
`init`/`handle`/`respond` rather than duplicating that machinery) but,
unlike the others, isn't part of the optional consumer-composed set —
`Engine::build` always registers it (ADR-020).

## What a module is

A native Rust crate compiled into the engine binary and registered in the
composition root. A module registers a **name** that becomes its bus endpoint,
in the same namespace as extensions and under the same uniqueness rule — on
the bus, modules and extensions are indistinguishable. The topic vocabulary is
the contract; whether `renderer` or `game-core` is native or WASM is a build
decision invisible to everyone else.

## Module or extension?

|                                        | Native module | WASM extension |
| -------------------------------------- | ------------- | -------------- |
| OS APIs, threads, native performance   | yes           | no             |
| Hot reload, sandboxing, any language   | no            | yes            |
| Trust required                         | full          | none           |
| Talks via                              | bus + service traits | bus only |

Rule of thumb: hot *infrastructure* native, hot *content* WASM — e.g. a
native game-core module with WASM level extensions.

## Module contract

A `Module` (crate: `bus`) requires `Handler` as a supertrait — a module is a
bus endpoint exactly like an extension, so `dispatch` and `tick` need no
separate hook: they already ride `Handler::handle` and a `core/tick`
subscription, the same as any extension.

- `name()` — the bus endpoint id, checked unique at registration.
- `init(ctx) -> Result<(), String>` — request subscriptions (applied by the
  caller after registration, mirroring how a WASM extension's own `init`
  requests them) and provide/consume services.
- `handle(&mut self, envelope)` (`Handler`) — bus deliveries, same semantics
  as extensions (per-module serialization included).
- `render()`, `present()` — frame-phase hooks, both default no-op; a module
  overrides only the phases it needs.
- `respond(sender, payload)` — answers a direct `send` (ADR-010) addressed
  to this module by name, the same capability WASM extensions already have.
  Default: no reply. `persistence` is the first module to use this (an
  extension's own `init` loading its prior save synchronously), and `files`
  the second (reading a granted directory); most modules have nothing to
  answer.
- `shutdown()` — called once after extensions stop during orderly application
  shutdown.

## Frame phases

The kernel's **runner** owns the loop skeleton. Modules hook named phases;
within one phase, registration order breaks ties.

| Phase      | Kernel work                            | Typical module hooks        |
| ---------- | -------------------------------------- | --------------------------- |
| `input`    | Platform pumps events → bus messages   | —                           |
| `dispatch` | Bus delivers to subscribers            | ui consumes `ui/*` specs (`Handler::handle`) |
| `tick`     | `on-tick(dt)` to tick subscribers      | game-core simulation (`Handler::handle` on `core/tick`) |
| `render`   | —                                      | renderer executes gfx batches, draws ui output |
| `present`  | —                                      | renderer presents the frame |

A headless build (no presentation modules) simply has empty `render` and
`present` phases.

## Services

The bus carries *behavior*; **services** carry in-process plumbing that must
not be per-frame message traffic. The registry is `TypeId`-keyed. `consume`
transfers ownership and removes a service; `get` borrows without claiming it.
Web borrows the SDL window and retains a cloned handle so its native child
views can follow the live client area after renderer consumes the original
handle. Providers register during engine construction or module `init`, and
consumers look up by type. The allowed services are enumerated here — adding
one is a design change, not a convenience:

| Service        | Provider          | Consumers      | Carries                     |
| -------------- | ----------------- | -------------- | --------------------------- |
| window-surface | platform (kernel), via `Engine::build` | renderer, web | `sdl3::video::Window` |
| bus            | kernel, via `Engine::build`        | game-core      | `bus::Bus` |
| draw-target    | *renderer module*  | *ui*           | *draw-data submission (egui triangles)* |

`window-surface` is real — web borrows it and `renderer`'s `init` consumes it.
`bus` is also real — unlike `renderer`/`ui`/`web`, which get a `Bus` handle
from their own builder sugar, a module injected via the generic
`.module(...)` path (game-core is the first that needs to publish) has no
other way to reach one, so `Engine::build` provides it as a service
unconditionally, the same as `window-surface`. `draw-target` is still
aspirational: `ui` direct-wires to `renderer`'s crate instead
(docs/structure.md) rather than consuming a service, pending its own
migration onto `Module`.

Modules never depend on each other's crates — only on kernel crates and on
services (which the kernel or a providing module defines by registering a
value of a given type). This is what lets an embedder swap the SDL renderer
for its own: any module providing the same service types slots in under an
unchanged consumer.

## Trust model

Modules are native and trusted: no time or queue budgets, no watchdog, no
quarantine, no hot reload. A module that stalls a frame phase stalls the
engine — that is the price of native, and the reason application logic should
default to extensions (ADR-007 protections) unless it needs what only a
module can have.

## Composition root and distributions

bones has two first-class distributions built from the same code path:

- **The app** — the engine executable composing the default SDL renderer and
  egui UI, plus web when built with the `web` feature and enabled in config.
  This is the main product and the common case: most projects take the app
  as-is and implement everything as WASM extensions — no Rust, no build of
  bones itself.
- **The library** — the workspace of kernel and module crates plus the
  builder API, for the embedding case: projects that need their own native
  modules (custom renderer, native game core) own the composition root — the
  `main` that wires kernel + modules — themselves.

The app is built solely on the public builder API with **no privileged
access**: if the app can't do something cleanly, embedders can't either, so
the API stays honest and the two distributions cannot drift.

Sketch of an embedder's composition root:

```rust
bones::Engine::new()
    .module(gpu_renderer::GpuRenderer::new())   // replaces the SDL renderer
    .module(bones_ui::EguiUi::default())        // reused from bones
    .module(game_core::GameCore::new())         // native-speed game logic
    .extensions_dir("levels/")                  // WASM levels, hot-reloadable
    .run()
```

## Embedding bones

The intended pattern is bones as a **git subrepo** (submodule) with path
dependencies, or a cargo git dependency — both build from source in one build
graph, so module injection is static dispatch with zero ABI machinery.
Consequences the embedder owns:

- **Pinning is the stability model**: the parent pins a bones commit and
  upgrades deliberately; no semver promise before 1.0 (ADR-011).
- The parent's toolchain and feature unification govern bones' build; the
  kernel brings wasmtime as an inherent dependency.
