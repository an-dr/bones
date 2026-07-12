# runner

The frame-phase loop and the public builder API (design/modules.md).

- `Runner` — step-driven primitive: `step(dt)` publishes `core/tick` and
  dispatches; `run_for(n, dt)` loops it. Bounded and deterministic — what
  tests use (ADR-014).
- `Engine` — the builder: `Engine::builder().extensions_dir(path).run()`
  discovers `.wasm` files, loads each via `host` (registering it on both the
  bus and a `bus::Registry`, so it's reachable by pub/sub and by direct
  `send`, ADR-010), subscribes it to whatever topics it requested during
  `init` (opt-in, including `core/tick`), publishes a `core/lifecycle`
  `Loaded`/`Faulted` event, and runs them over real wall-clock timing
  (default 60Hz, override with `.tick_hz(hz)`). A thin wrapper around
  `Runner::step`, not a second primitive.
- `.window(title, w, h)` — opens one SDL window (`core/platform`); its
  keyboard events are polled onto the bus once per loop iteration, before
  `step`'s dispatch.
- `.renderer()` — attaches a `Renderer` (`core/renderer`) to the window;
  presents once per loop iteration after `step`'s dispatch.
- `Supervisor` — sweeps loaded extensions each check for one that's faulted
  (ADR-007's time/queue budgets, enforced inside `host::Host`) or whose
  `.wasm` file changed (design/extensions.md's Reloading state), and reacts:
  quarantines a fault (drops its bus registration, publishes `Faulted`) or
  hot-swaps a changed file in place (drop old, load new, publish
  `Reloading`/`Reloaded`). A replacement that fails to load is logged and
  whatever was running keeps running. The engine, and every other
  extension, keeps going regardless.
- `.build()` returns `(Runner, Option<Platform>, Option<Arc<Mutex<Renderer>>>,
  Supervisor)` so a future driver can use all of them without `run`'s
  sleep-loop attached; `run` calls `Supervisor::check` both before and after
  `step` each iteration — before so a swapped-in extension's first tick
  already runs against the new code, after so a fault from that tick is
  quarantined the same iteration.
