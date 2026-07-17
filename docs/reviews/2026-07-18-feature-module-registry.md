# Code review — 2026-07-18, feature/module-registry

## Summary

Adds the `Module` trait and typed `ServiceRegistry` (`core/bus/src/module.rs`,
ADR-017), migrates `Renderer` onto it with two-stage construction, wires
`.module(...)` into `Engine::build`/`run`, and adds `embedding-demo/` proving
a separate crate can inject a native module with no privileged access.
Overall quality is solid: the design matches docs/design/modules.md's
already-specced contract, and both new integration tests
(`a_window_with_no_renderer_or_module_is_not_dropped_with_the_service_registry`,
`a_custom_module_can_consume_window_surface_without_renderer`) target the one
genuinely subtle area in this diff. Zero critical or high findings survive.

One real defect was caught and fixed during this same pass, before this
review was formalized, so it does not appear below as an open finding:
`window-surface` was originally only seeded into the registry inside the
`.renderer()` branch, meaning (a) a `.module(...)`-injected replacement
renderer had no way to get the window at all (contradicting the "no
privileged access" goal this rung exists to prove), and (b) dropping the
registry with an unconsumed window would have closed it. Fixed by seeding
the registry unconditionally and adding `Platform::provide_window`/
`reclaim_window` (`core/platform/src/lib.rs:52-69`,
`core/runner/src/engine.rs:238-241,277-279`), with the two tests above
covering both halves.

## Findings

### Critical — must fix before merge

None.

### High

None.

### Improvements

- **CR.1** `core/bus/src/module.rs:65-72` — `ServiceRegistry::provide`
  panics (via `assert!`) on a duplicate registration rather than returning
  a `Result`. Consistent with the existing native-module trust model
  (modules.md: "no watchdog, no quarantine" — a module bug is already
  uncontained), so not a regression, but worth knowing: a duplicate
  `provide` aborts the whole `Engine::build`, not just that one module.
  Better if it ever needs to become recoverable: return
  `Result<(), ServiceAlreadyProvided>` and let `register_module` propagate
  it the same way `init` errors already do.

- **CR.2** `core/renderer/examples/render_smoke.rs:16`,
  `core/ui/examples/ui_smoke.rs:20` — both call
  `Module::init(&mut renderer, &mut ctx)?` (fully-qualified) where
  `renderer.init(&mut ctx)?` resolves identically, since `Module` is
  imported and `Renderer` has no inherent `init` to disambiguate against.
  Cosmetic; the fully-qualified form isn't wrong, just unnecessary here.

- **CR.3** `core/runner/src/engine.rs:342-362` — `run`'s loop locks every
  entry in `modules` twice per frame (once for `render`, once for
  `present`), not once. Deliberate — it keeps the phase boundary real (every
  module's `render` completes before any `present` starts, matching design/
  modules.md's phase table) rather than interleaving per-module — but worth
  knowing if the module count ever grows enough for lock overhead to matter;
  no module today does anything expensive enough for it to show up.

- **CR.4** `core/runner/src/engine.rs:42-53` — `SharedModule` duplicates
  `Shared<T>`'s shape almost exactly; the code comment already explains why
  it can't be `Shared<Box<dyn Module>>` (a `Box<dyn Module>: Handler` impl
  would conflict with `bus`'s existing blanket `impl<F: FnMut...> Handler
  for F`, a coherence conflict, confirmed by trying it). Not fixable without
  specialization (unstable) or reworking `Module`'s `Handler` supertrait
  relationship — noting only so a future reader doesn't attempt the same
  merge expecting it to work.

## Positives

- `ServiceRegistry`'s single-consumer (remove-on-read) semantics are a
  deliberately small, honest scope cut — documented inline as a TODO tied to
  the specific future trigger (`web` needing `window-surface` too) rather
  than either over-building `Arc`-sharing now or leaving the limitation
  unstated.
- The `window-surface` provide/reclaim round trip is exercised by real
  integration tests exactly at the two failure modes that matter (window
  dropped when unclaimed; custom module denied the service `.renderer()`
  gets) rather than only at the happy path.
- `embedding-demo` is a genuinely separate `[workspace]` with path
  dependencies, not a member of the root workspace — it actually exercises
  the "external consumer" scenario design/modules.md describes, not a
  same-workspace stand-in for one.

## Verdict

Approve. The four improvements are all non-blocking (style, a documented
trade-off, and a documented Rust-coherence constraint) — none change
behavior or leave a defect in place.
