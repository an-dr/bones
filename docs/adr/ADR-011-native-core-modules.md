# ADR-011: Native core modules — kernel plus consumer-composed modules

## Problem

The core is a fixed set of components wired by an internal composition root. `web` is optional by special-case rule; nothing else is. Embedders — e.g. a game engine adding bones as a subrepo — need to add native-speed capabilities (their own renderer, a native game core) without forking, and minimal or headless builds need to drop presentation entirely. There is no stated mechanism for either.

## Decision

The core splits into two tiers:

- **Kernel** — bus, host, contract, platform, logging, and the **runner** (frame-phase loop skeleton, module and service registries, builder API). Always present.
- **Native modules** — renderer, ui, web today; audio, networking, or an embedder's game core later. A module is a Rust crate implementing the module contract ([design/modules.md](../design/modules.md)), registered at build time through the builder. Every module is optional.

Two first-class distributions share one code path. **The app** — the engine executable composing the default modules — is the main product: most projects use it as-is and implement only WASM extensions. **The library** serves embedders that need native modules: they add bones as a subrepo (or git dependency) and own the composition root — the `main` that wires kernel + modules — in their own binary. The app is built solely on the public builder API with no privileged access, so the two distributions cannot drift.

Modules are **bus endpoints in the same namespace as extensions** and are otherwise indistinguishable from them on the bus. They are **trusted**: no time/queue budgets, no quarantine, no hot reload. For in-process plumbing that must not be bus traffic (window surfaces, draw targets), modules use a small enumerated set of **service traits** (design/modules.md keeps the inventory).

**Static linking only** — no dynamic loading. API stability is per pinned commit; no semver promise before 1.0.

## Rationale

- Module/extension symmetry on the bus gives a promotion path — prototype a capability as a WASM extension, move it to a native module when speed demands — and swappable backends: any module speaking the `ui/*` vocabulary can replace egui without touching anything else.
- One build graph with static dispatch: native speed and no stable-ABI problem, which Rust dynamic loading cannot offer.
- The app as an ordinary consumer of the builder API keeps that API honest and prevents drift between embedded and standalone bones — while staying the zero-Rust product most extension-only projects need.

## Rejected alternatives

- **Dynamic loading (dylib plugins)** — Rust has no stable ABI; pins compiler versions and demands unsafe glue forever, for a flexibility static composition already provides here.
- **Everything stays a WASM extension** — the sandbox and marshalling tax rules out native-speed renderers and OS-API integrations (webviews, audio devices, GPU access).
- **Fixed core with more feature flags** — subtraction only; embedders could remove capabilities but never add their own without forking.
