# kernel

The one crate an embedder depends on. Everything reachable from `bones-kernel` is public API; everything under `crates/` that it does not re-export is an implementation detail.

That distinction is enforced by the dependency graph rather than by convention. `bones` — the engine executable — depends on this crate and on nothing else here, so architecture/structure.md's rule that the app has no access an embedder lacks cannot quietly stop being true.

## Public shape

- Root — `Kernel`, `BuiltKernel`, `Runner`, `Supervisor`, `read_tick_dt`, and the `Error`/`Result` pair the fallible calls return.
- `bus` — `Module`, `ModuleContext`, `Handler`, `Envelope`, the service registry, the budget types, and (with `presentation`) `PlatformEvent` plus `offer_event`.
- `platform` (with `presentation`) — `Platform`, and `WindowSurface` for the `window-surface` service.
- `draw_target` (with `presentation`) — `DrawTarget`, `DrawTargetService`, `UiMesh`, `UiVertex`.
- `logging` — `Logger`, `LogSink`, `Level`, and the two shipped sinks.
- `messages` — the typed core messages, re-exported from `bones-messages`.
- `audio`, `game_core` — the optional native modules, behind features of the same name.
- `window_control` — `WindowControl`, the `window-control` service a module reads to bring the window to the front without owning it.
- `instance`, `desktop`, `upgrade`, `devtools` — what the application-shell builder methods take, behind features of the same name: `.single_instance(primary)` with a `Primary` from `instance::claim`, `.desktop(integration)`, `.upgrade(config)`, and `.devtools_from_env(variable)`.
- `launch`, `watch` — plain libraries for an application's native code, behind features of the same name: starting programs and terminals, and debounced folder watching. No bus module.

The surface is curated, not a glob. Adding to it is deliberate: what appears here is what the engine promises to keep working. Every public item carries documentation, and `missing_docs` is enabled so a new one cannot arrive without it.

`BuiltKernel`'s fields are public and stable, not an accident of visibility. An embedder driving its own loop needs all of them at once and takes them by destructuring; accessors would each borrow the whole value and make that impossible.

## Constraints worth knowing

Presentation *backends* are absent on purpose. `renderer`, `ui`, and `web` are selected through builder methods — `.renderer()`, `.ui()`, `.web()` — so an embedder composes them without ever naming their types, and they stay private.

What an embedder must be able to *name*, it can. `bus::PlatformEvent` is the argument of the hook a custom module overrides, `platform::Platform` is a value `BuiltKernel` hands back, and `Error` is what `build` and `run` fail with — so none of them require adding `sdl3`, `wasmtime`, or `bones-kernel-infra` to a consumer's manifest.

When adding a public signature that mentions a type from a crate beneath this one, re-export that type here too. The crate-level and `platform` documentation carry compiled examples that name every such type through `bones_kernel`, so deleting a re-export stops the doc tests building rather than surfacing later as an embedder's problem. [examples/embedding/custom-engine](../../examples/embedding/custom-engine/README.md) is the fuller check — it depends on `bones-kernel` alone, so a type it cannot name is a type no embedder can name.

`messages` re-exports a crate on the **ABI version line**, not this crate's engine version. `bones-messages` moves only when the guest contract changes, which is why a native module and a WASM guest can share one vocabulary.

Feature gates are forwarded, not redefined. `presentation` and `web` map straight onto `bones-kernel-infra`'s own gates (ADR-027); `audio`, `game-core`, `os`, `instance`, `desktop`, `upgrade`, `devtools`, `launch`, and `watch` toggle optional dependencies; every one after `game-core` is off by default, because each is something an application chooses to be rather than something an engine is. Building with `--no-default-features` yields a headless engine with no presentation stack, which is the shape ADR-014 and ADR-028 exist for.
