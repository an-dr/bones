# ADR-016: `bones-*` crate names plus a `bones` facade crate

## Problem

Every workspace crate (`bus`, `host`, `contract`, `platform`, `runner`,
`lifecycle`, `logging`, `renderer`) carried an unprefixed generic name.
Workable as private workspace members, but design/modules.md commits to a
**library distribution** consumed as a git dependency (subrepo or git
dep) — at which point an embedder's own `Cargo.toml` collides with any
other dependency, in their workspace or on crates.io, named `logging` or
`platform`. `runner::Engine` also stood in for the design sketch's
`bones::Engine` with no facade crate to make that literal.

## Decision

- Rename every kernel crate with a flat `bones-` prefix: `bones-bus`,
  `bones-host`, `bones-contract`, `bones-platform`, `bones-runner`,
  `bones-lifecycle`, `bones-logging`. No `core` infix — it would add a
  hierarchy level with no information; the `core/` *directory* is
  unaffected, only `[package] name` changes.
- Same prefix for native modules: `bones-renderer` (and `bones-ui`,
  `bones-web` when they exist). No `module` infix — ADR-011 makes the
  module/extension distinction invisible on the bus; names shouldn't bake
  it in.
- Add a thin **`bones` facade crate** (`core/bones`) — the only name an
  embedder depends on. Re-exports `Engine`, `BuiltEngine`, `Supervisor`
  from `bones-runner`. `app` depends on `bones`, not on `bones-runner`
  directly, so the app has no access an embedder using the facade lacks
  (structure.md's existing rule, now enforced by the dependency graph
  itself).
- `app` keeps its own package name (`app`, binary `bones`) — it is the
  final executable, never a library dependency of anything else, so the
  collision risk this ADR addresses doesn't apply to it.
- `shared/bones-vocab` (formerly `buffer_rw`) already carried the prefix
  from its own introduction; unaffected by this ADR.
- `vendor/pubsub-bus` is a third-party dependency, not a bones crate;
  unaffected.

## Rationale

- A rename is mechanical today (workspace-internal path dependencies) and
  an ecosystem-facing breaking change once the library distribution is
  consumed externally — doing it now, before rung 1 (ui module) adds more
  crates, is the cheaper time to pay for it.
- The facade crate is what makes `app`'s "no privileged access" rule
  (structure.md) load-bearing rather than aspirational: before this ADR,
  nothing stopped `app` from reaching past the intended public surface
  into `bones-runner`'s internals, since it depended on the crate
  directly.

## Rejected alternatives

- **A single `bones-core` crate exposing everything** — considered during
  the same review this ADR closes out. Rejected: it would turn the
  compiler-enforced dependency rules (structure.md's dependency diagram)
  into unenforced convention, would put SDL/wry behind feature flags of
  the one crate every embedder's kernel sits in, and still couldn't
  absorb `bones-vocab` (which must compile to `wasm32-wasip2` without
  wasmtime or SDL as transitive dependencies).
- **Leave the facade for later, rename crates only** — would leave the
  `bones::Engine` TODO unresolved and `app` still holding direct access to
  `bones-runner`, the exact asymmetry structure.md's "no privileged
  access" rule exists to prevent.
