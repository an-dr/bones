# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Bounded and deterministic — what
  tests use (ADR-014).
- `Engine` — the builder: `Engine::builder().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host`, subscribes it to whatever
  topics it requested during `init` (opt-in, including `core/tick`), and
  runs them over real wall-clock timing (default 60Hz, override with
  `.tick_hz(hz)`). A thin wrapper around `Runner::step`, not a second
  primitive.
- `.window(title, w, h)` — opens one SDL window (`core/platform`); its
  keyboard events are polled onto the bus once per loop iteration, before
  `step`'s dispatch. `.build()` returns `(Runner, Option<Platform>)` so a
  future driver can use both without `run`'s sleep-loop attached.
