# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Bounded and deterministic — what
  tests use (ADR-014).
- `Engine` — the builder: `Engine::builder().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host`, subscribes it to whatever
  topics it requested during `init` (opt-in, including `core/tick`),
  publishes a `core/lifecycle` `Loaded`/`Faulted` event, and runs them over
  real wall-clock timing (default 60Hz, override with `.tick_hz(hz)`). A
  thin wrapper around `Runner::step`, not a second primitive.
- `.window(title, w, h)` — opens one SDL window (`core/platform`); its
  keyboard events are polled onto the bus once per loop iteration, before
  `step`'s dispatch.
- `.renderer()` — attaches a `Renderer` (`core/renderer`) to the window;
  presents once per loop iteration after `step`'s dispatch.
- `Watchdog` — sweeps loaded extensions each iteration for one that's
  faulted (ADR-007's time/queue budgets, enforced inside `host::Host`) and
  quarantines it: drops its bus registration, publishes `Faulted`. The
  engine, and every other extension, keeps running.
- `.build()` returns `(Runner, Option<Platform>, Option<Arc<Mutex<Renderer>>>,
  Watchdog)` so a future driver can use all of them without `run`'s
  sleep-loop attached; `run` polls the window, steps, checks the watchdog,
  and presents, once per iteration, in that order.
