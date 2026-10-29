# Native modules and the composition root

Detailed design of the module system. Decisions: [ADR-011](../adr/ADR-011-native-core-modules.md) (kernel infrastructure + consumer-composed modules), [ADR-017](../adr/ADR-017-native-module-trait-and-typed-service-registry.md) (the concrete `Module` trait and service registry), [ADR-031](../adr/ADR-031-native-modules-reach-each-other-only-through-services.md) (modules reach each other only through services). All six shipped modules implement this contract for real, and all six arrive through `.module(...)` — `.renderer()`, `.ui()`, `.web()`, and `.os()` are sugar over it, not a second route in. `wasm-extensions::persistence` also implements it (reusing `init`/`handle`/`respond` rather than duplicating that machinery) but, unlike the others, isn't part of the optional consumer-composed set — `Kernel::build` always registers it (ADR-020).

## What a module is

A native Rust crate compiled into the engine binary and registered in the composition root. A module registers a **name** that becomes its bus endpoint, in the same namespace as extensions and under the same uniqueness rule — on the bus, modules and extensions are indistinguishable. The topic vocabulary is the contract; whether `renderer` or `game-core` is native or WASM is a build decision invisible to everyone else.

## Module or extension?

| | Native module | WASM extension |
| --- | --- | --- |
| OS APIs, threads, native performance | yes | no |
| Hot reload, sandboxing, any language | no | yes |
| Trust required | full | none |
| Talks via | bus + service traits | bus only |

Rule of thumb: hot *infrastructure* native, hot *content* WASM — e.g. a native game-core module with WASM level extensions.

## Module contract

A `Module` (crate: `bus`) requires `Handler` as a supertrait — a module is a bus endpoint exactly like an extension, so `dispatch` and `tick` need no separate hook: they already ride `Handler::handle` and a `core/tick` subscription, the same as any extension.

- `name()` — the bus endpoint id, checked unique at registration.
- `init(ctx) -> Result<(), String>` — request subscriptions (applied by the caller after registration, mirroring how a WASM extension's own `init` requests them) and provide/consume services.
- `handle(&mut self, envelope)` (`Handler`) — bus deliveries, same semantics as extensions (per-module serialization included).
- `filter_event(event) -> bool` — first look at one raw platform event before it becomes an `input/*` message (ADR-008); returning `true` claims it so nothing below sees it. Offered in reverse registration order, so the module composed last — the one drawn on top — is asked first (ADR-031). Default: claims nothing. Feature-gated with `platform`, since a headless build has no event source.
- `render()`, `present()` — frame-phase hooks, both default no-op; a module overrides only the phases it needs.
- `respond(sender, payload)` — answers a direct `send` (ADR-010) addressed to this module by name, the same capability WASM extensions already have. Default: no reply. `persistence` is the first module to use this (an extension's own `init` loading its prior save synchronously), and `files` the second (reading a granted directory); most modules have nothing to answer.
- `shutdown()` — called once after extensions stop during orderly application shutdown.

## Frame phases

The kernel infrastructure's **runner** owns the loop skeleton. Modules hook named phases; within one phase, registration order breaks ties.

| Phase | Kernel infrastructure work | Typical module hooks |
| --- | --- | --- |
| `input` | Platform pumps events → bus messages | every module's `filter_event`, topmost first, first claim wins |
| `before tick` | — | `before_tick`: the wasm module applies hot reloads and runtime commands, so the coming tick runs against the new code |
| `dispatch` | Bus delivers to subscribers | ui consumes `ui/*` specs (`Handler::handle`) |
| `tick` | `on-tick(dt)` to tick subscribers | game-core simulation (`Handler::handle` on `core/tick`) |
| `after tick` | — | `after_tick`: the wasm module quarantines an extension that faulted during this tick, this frame rather than the next |
| `render` | — | renderer composites gfx batches, then ui draws above them |
| `present` | — | renderer presents the frame |

While `frame-control` holds the simulation paused, the loop still pumps input, dispatches the bus, and runs `render` and `present`. The window stays responsive, a request such as a capture reaches its module, and a screenshot shows the paused frame. It skips the tick and the `before tick` and `after tick` hooks, so a runtime extension command waits for the next step. Each requested step runs one full frame with `dt` equal to the configured tick period rather than the time measured since the last frame, so a stepped run is repeatable.

Registration order is what layers the frame: `render` runs over every module before `present` runs over any, so the renderer has composited before ui draws, and nothing flips a buffer until both are done. A headless build (no presentation modules) simply has empty phases.

Shutdown has one more hook. On a close request the runner dispatches it, then calls every module's `begin_shutdown` and dispatches again, then every module's `shutdown`. `begin_shutdown` is where a module stops what other modules may still need to hear from: the wasm module calls each extension's own `shutdown` there, so the messages those calls publish reach modules that have not shut down yet. The runner names no module in any of this ([ADR-040](../adr/ADR-040-the-wasm-extension-host-is-an-opt-in-module.md)).

The `input` phase walks that same order backward (ADR-031). Later registration means drawn above, so it has to mean offered input first — the two directions are one statement about layering read from opposite ends, and an embedder sets both by choosing where in the composition a module goes.

## Services

The bus carries *behavior*; **services** carry in-process plumbing that must not be per-frame message traffic. The registry is `TypeId`-keyed. `consume` transfers ownership and removes a service; `get` borrows without claiming it. Web borrows the SDL window and retains a cloned handle so its native child views can follow the live client area after renderer consumes the original handle. Providers register during engine construction or module `init`, and consumers look up by type. The allowed services are enumerated here — adding one is a design change, not a convenience:

| Service | Provider | Consumers | Carries |
| --- | --- | --- | --- |
| window-surface | platform (kernel infrastructure), via `Kernel::build` | renderer, web | `sdl3::video::Window` |
| bus | kernel infrastructure, via `Kernel::build` | game-core | `bus::Bus` |
| draw-target | renderer, via its own `init` | ui | `Box<dyn DrawTarget>`: textures plus `UiMesh` submission |
| window-control | platform (kernel infrastructure), via `Kernel::build` | instance | `WindowControl`: `raise()` — un-minimise, raise, and focus the window, where the platform allows (ADR-035) |
| frame-control | kernel infrastructure, via `Kernel::build` | devtools | `FrameControl`: pause the simulation, resume it, or step it a number of frames at the fixed tick period |

`window-surface` is provided by the builder — web borrows it and `renderer`'s `init` consumes it. `bus` is provided by the builder unconditionally: unlike `renderer`/`ui`/`web`, which get a `Bus` handle from their own builder sugar, a module injected via the generic `.module(...)` path (game-core is the first that needs to publish) has no other way to reach one. `window-control` exists because `window-surface` cannot serve a module that only needs to act on the window: the renderer consumes the window outright, and the window cannot leave the frame thread. `window-control` is a cloneable handle that carries the capability without the window, read with `get` and never consumed, so any number of modules can hold one; its calls run on the frame thread, which is where every module hook already runs. `draw-target` is the first service one *module* provides to another: `renderer` registers it during its own `init`, and `ui` consumes it during its, so neither builder nor consumer names the other's crate (ADR-031).

Modules never depend on each other's crates — only on kernel infrastructure crates and on services (which the kernel infrastructure or a providing module defines by registering a value of a given type). This is what lets an embedder swap the SDL renderer for its own: any module providing the same service types slots in under an unchanged consumer.

## Trust model

Modules are native and trusted: no time or queue budgets, no watchdog, no quarantine, no hot reload. A module that stalls a frame phase stalls the engine — that is the price of native, and the reason application logic should default to extensions (ADR-007 protections) unless it needs what only a module can have.

## Composition root and distributions

bones has two first-class distributions built from the same code path:

- **The app** — the engine executable composing the default SDL renderer and egui UI, plus web when built with the `web` feature and enabled in config. This is the main product and the common case: most projects take the app as-is and implement everything as WASM extensions — no Rust, no build of bones itself.
- **The library** — the workspace of kernel infrastructure and module crates plus the builder API, for the embedding case: projects that need their own native modules (custom renderer, native game core) own the composition root — the `main` that wires kernel infrastructure + modules — themselves.

The app is built solely on the public builder API with **no privileged access**: if the app can't do something cleanly, embedders can't either, so the API stays honest and the two distributions cannot drift.

Sketch of an embedder's composition root:

```rust
bones::Kernel::new()
    .module(gpu_renderer::GpuRenderer::new())   // replaces the SDL renderer
    .module(bones_ui::EguiUi::default())        // reused from bones
    .module(game_core::GameCore::new())         // native-speed game logic
    .extensions_dir("levels/")                  // WASM levels, hot-reloadable
    .run()
```

## Embedding bones

The intended pattern is bones as a **git subrepo** (submodule) with path dependencies, or a cargo git dependency — both build from source in one build graph, so module injection is static dispatch with zero ABI machinery. Consequences the embedder owns:

- **Pinning is the stability model**: the parent pins a bones commit and upgrades deliberately; no semver promise before 1.0 (ADR-011).
- The parent's toolchain and feature unification govern bones' build. wasmtime comes only with the `wasm` feature, which hosts extensions; an application that needs none leaves it off (ADR-040).
