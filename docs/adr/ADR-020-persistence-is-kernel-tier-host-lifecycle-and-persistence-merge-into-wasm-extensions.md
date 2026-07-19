# ADR-020: Persistence is kernel-tier; host, lifecycle, and persistence merge into wasm-extensions

## Problem

[ADR-018](ADR-018-core-2d-presentation-input-and-persistence-gaps.md) framed
`persistence` as "a new native module... same trust tier as `renderer`/`ui`"
— optional, toggled via `bones.toml`, its own crate. In review, that framing
didn't hold up: unlike `audio`/`renderer` (a real environment might lack the
hardware) or a hypothetical size-gated module (real dependency weight to
shed), disabling `persistence` saves nothing — it has no dependency beyond
`std::fs`, and its resource (a writable directory) exists on every
environment bones targets. `Config::persistence: bool` was a toggle with no
actual resource or size justification behind it.

Separately, `host` (loading, dispatch, watchdog) and `lifecycle`
(state-transition events) — both always-on kernel components, never
`Module`-trait implementors, never toggleable — sit conceptually next to
`persistence`: all three are about an extension's existence over time
(load it, track its state, remember its state), as opposed to
`renderer`/`ui`/`audio`, which are about what an extension can currently
*do*. They lived in three separate crates for no reason connected to that
grouping.

## Decision

- `persistence` becomes unconditional: `Engine::build` always constructs
  and registers it, with no `bones.toml` toggle to skip it. It still
  implements the `Module` trait (reusing `init`/`handle`/`respond` rather
  than duplicating that machinery), but it is not "a module" in
  [ADR-011](ADR-011-native-core-modules.md)'s optional, consumer-composed
  sense — it's kernel infrastructure that happens to reuse `Module`'s
  shape. `docs/structure.md`'s "every module is optional" principle refers
  to that optional, consumer-composed set; `persistence` was moved out of
  it, not an exception carved into it.
- What stays configurable is *read-only mode*
  (`Engine::read_only_persistence`, `bones.toml`'s
  `persistence_read_only`): extensions can still load previously-saved
  state, but new saves are silently dropped. That's a policy choice (an
  audited or locked-down extension sandbox), not a resource one — the
  distinction the original toggle conflated.
- `core/host`, `core/lifecycle`, and `core/persistence` merge into one
  crate, `core/wasm-extensions`, with `host`/`lifecycle`/`persistence` as
  its submodules. `audio` (a real optional module — real dependency
  weight, a real environment-might-lack-it resource) stays its own crate,
  unaffected.

## Rationale

- Resource-availability and dependency-weight are the load-bearing reasons
  `audio`/`renderer` are optional; neither applies to `persistence`, so
  keeping it in that category was cargo-culting the pattern rather than
  applying its actual reasoning.
- `host`/`lifecycle`/`persistence` sharing a crate makes the "extension
  existence over time" grouping real (a directory a reader can find, not
  just a sentence in a doc) at zero cost — `persistence` adds no
  dependency `host` doesn't already dwarf (`wasmtime`, `wasmtime-wasi`).
- Keeping `persistence` `Module`-shaped (instead of hand-rolling bespoke
  init/handle/respond wiring the way `host` predates the `Module` trait
  and never adopted it) reuses tested machinery for free; unconditional
  registration is just a different *caller* (`Engine::build` itself,
  rather than `self.modules`), not a different contract.

## Rejected alternatives

- **Keep the `persistence: bool` toggle** — the option existed, but
  disabling it bought literally nothing (no size, no speed, and the
  resource it needs is universal), so it was a footgun (an embedder
  forgets to enable it, extension state silently never loads) with no
  offsetting benefit. Read-only mode is the config knob that actually
  corresponds to a real reason to change persistence's behavior.
- **Name the merged crate `extensions`** — collides with the existing,
  different meaning of "extensions" in this repo (the top-level
  `extensions/` directory of sandboxed WASM guest content) — confusing to
  read as "the crate that *is* extensions" vs. its actual job, managing
  them.
- **Leave `host`/`lifecycle`/`persistence` as separate crates** — no
  dependency-weight or optionality reason to keep them apart once
  `persistence` is kernel-tier too; the grouping is real, so make it legible
  in the source layout, not just in prose.
