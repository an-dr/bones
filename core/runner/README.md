# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Bounded and deterministic — what
  tests use (ADR-014).
- `Engine` — the builder: `Engine::builder().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host`, and runs them over real
  wall-clock timing (default 60Hz, override with `.tick_hz(hz)`). A thin
  wrapper around `Runner::step`, not a second primitive.
