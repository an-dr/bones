# ADR-017: Native module trait and typed service registry

## Problem

ADR-011 established the two-tier model (fixed kernel, optional native
modules) but left the concrete shape open. Today `renderer` and `ui` are
hardwired straight into `Engine::build` — no embedder can inject their own
native module (e.g. a game-core), which is the whole point of the
"library" distribution (structure.md). Something has to define what a
module *is* in Rust terms, how it gets frame-phase hooks, and how it finds
kernel-provided or sibling-module capabilities (`window-surface`,
`draw-target`) without depending on the provider's crate directly.

## Decision

- **`Module` trait**, requiring `bus::Handler` as a supertrait (a module is
  a bus endpoint like an extension, per structure.md): `name(&self)`,
  `init(&mut self, ctx: &mut ModuleContext)`, and default-no-op
  `render(&mut self)`, `present(&mut self)`, `shutdown(&mut self)` —
  a module hooks only the frame phases it needs (design/modules.md's phase
  table). `dispatch` and `tick` need no separate hook: they already ride
  the bus (`Handler::handle`, `core/tick` subscription), matching how
  extensions work today.
- **`ModuleContext`**, passed to `init`, offers `subscribe(topic)`
  (applied after registration, mirroring how a WASM extension's `init`
  requests subscriptions today) and `provide_service<T>(value)` /
  `consume_service<T>() -> Option<T>` against a `TypeId`-keyed registry.
- **`Engine::module(impl Module)`**: modules run their `init` in
  registration order at `build()` time, after the registry is seeded with
  `window-surface` (from `platform`, if a window was configured).
  Registration order is the caller's contract for service dependencies —
  a module consuming a service another module provides must be registered
  after it.
- `.renderer()`/`.ui()` become sugar over `.module(...)`, not a separate
  code path — keeps `app`'s default composition proof that the public
  builder API needs no privileged access (modules.md).

## Rationale

- Reuses `Handler` instead of inventing a second message-dispatch
  mechanism — one bus-endpoint contract for extensions and modules alike.
- A `TypeId`-keyed registry is the smallest mechanism that lets an
  embedder's own module both provide and consume services bones doesn't
  know about in advance, which named getter methods on `Engine` couldn't.
- Explicit registration-order dependency (rather than a dependency graph
  or lazy resolution) matches the project's stated pre-1.0 stance: the
  simplest thing that works, not the most general.

## Rejected alternatives

- **Narrow per-capability marker traits, downcast via `Any`** — more
  composable, no no-op boilerplate for hooks a module doesn't need, but
  more machinery for a small win at this scale; revisit if the module
  count grows enough that empty `render`/`present` overrides get noisy.
- **Named getter methods on `Engine` per known service** (`window_surface()`,
  `draw_target()`) instead of a generic registry — simpler, no `TypeId`
  machinery, but every new service needs bespoke `Engine` plumbing and an
  embedder's own custom service has nowhere to register.
- **Lazy/graph-resolved service dependencies** — no registration-order
  footgun, but real dependency-resolution machinery for a problem two
  services (`window-surface`, `draw-target`) don't yet justify.
