# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Each step restores the configured
  per-extension inbound and publish allowances before delivery. Bounded and
  deterministic — what tests use (ADR-014).
- `Engine` — the builder: `Engine::new().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host`, and runs them over real
  wall-clock timing (default 60Hz, override with `.tick_hz(hz)`).
  `.window(title, w, h)` opens an SDL window; `.renderer()` attaches a
  renderer to it; `.extension_budget(limits)` configures message flow
  control. `run` is a thin wrapper around `Runner::step`, not a second
  primitive, and exits when the window is closed. Native presentation is
  enabled by the default `presentation` feature; service embedders can disable
  default features to compile a runner without SDL, renderer, or UI.
- `Supervisor` — sweeps loaded extensions each check for one that's faulted
  (ADR-007) or whose `.wasm` file changed (design/extensions.md's
  Reloading state), and reacts: quarantines a fault or hot-swaps a changed
  file in place. The engine, and every other extension, keeps going
  regardless.
- `.build()` returns a `BuiltEngine` (`runner`, optional presentation fields,
  `supervisor`) so an external driver can use the wired-up pieces without
  `run`'s sleep-loop attached. `BuiltEngine::is_headless()` reports whether
  native presentation was composed.
