# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Bounded and deterministic — what
  tests use (ADR-014).
- `Engine` — the builder: `Engine::new().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host`, and runs them over real
  wall-clock timing (default 60Hz, override with `.tick_hz(hz)`).
  `.window(title, w, h)` opens an SDL window; `.renderer()` attaches a
  renderer to it. `run` is a thin wrapper around `Runner::step`, not a
  second primitive, and exits when the window is closed.
- `Supervisor` — sweeps loaded extensions each check for one that's faulted
  (ADR-007) or whose `.wasm` file changed (design/extensions.md's
  Reloading state), and reacts: quarantines a fault or hot-swaps a changed
  file in place. The engine, and every other extension, keeps going
  regardless.
- `.build()` returns a `BuiltEngine` (`runner`, `platform`, `renderer`,
  `supervisor`) so a future driver can use the wired-up pieces without
  `run`'s sleep-loop attached.
