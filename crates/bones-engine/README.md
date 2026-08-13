# engine

The one crate an embedder depends on. Everything reachable from `bones-engine` is public API; everything under `crates/` that it does not re-export is an implementation detail.

That distinction is enforced by the dependency graph rather than by convention. `bones` — the engine executable — depends on this crate and on nothing else here, so structure.md's rule that the app has no access an embedder lacks cannot quietly stop being true.

## Public shape

- Root — `Engine`, `BuiltEngine`, `Runner`, `Supervisor`, `read_tick_dt`.
- `bus` — `Module`, `ModuleContext`, `Handler`, `Envelope`, the service registry, and the budget types.
- `logging` — `Logger`, `LogSink`, `Level`, and the two shipped sinks.
- `messages` — the typed core messages, re-exported from `bones-messages`.
- `audio`, `game_core` — the optional native modules, behind features of the same name.

The surface is curated, not a glob. Adding to it is deliberate: what appears here is what the engine promises to keep working.

## Constraints worth knowing

Presentation backends are absent on purpose. `renderer`, `ui`, `web`, and `platform` are selected through builder methods — `.renderer()`, `.ui()`, `.web()` — so an embedder composes them without ever naming their types, and they stay private.

`messages` re-exports a crate on the **ABI version line**, not this crate's engine version. `bones-messages` moves only when the guest contract changes, which is why a native module and a WASM guest can share one vocabulary.

Feature gates are forwarded, not redefined. `presentation` and `web` map straight onto `bones-kernel`'s own gates (ADR-027); `audio` and `game-core` toggle optional dependencies. Building with `--no-default-features` yields a headless engine with no presentation stack, which is the shape ADR-014 and ADR-028 exist for.
