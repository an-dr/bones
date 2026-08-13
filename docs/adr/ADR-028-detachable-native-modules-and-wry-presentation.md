# ADR-028: Detachable native modules and wry presentation

## Problem

Service embedders keep a Bones engine alive without presentation, but may need to open a native web surface temporarily. Building a second engine would split the bus and extension catalog, while configuring the existing engine with a window would keep SDL and wry resources alive for the service lifetime.

## Decision

- Native modules may attach to and detach from a live bus and direct-call registry through a lifecycle-owning registration.
- Detach runs module shutdown and removes both endpoint registrations.
- The optional web crate provides a detachable wry presentation that owns its SDL parent window and `web` module only while open.
- The presentation shares the embedder's existing bus and registry; it does not convert or rebuild the owning engine.
- Closing is idempotent, destroys native presentation resources, and permits a later presentation to reuse the `web` endpoint.

## Rationale

The headless runtime remains the single owner of extensions and application state. Presentation becomes a temporary native module composition over the same message fabric, so opening a window neither restarts the runtime nor introduces a second transport.

## Rejected alternatives

- **Keep a hidden startup window** — retains presentation resources while idle and makes the engine operationally non-headless.
- **Build a second windowed engine** — duplicates extension lifecycle and separates UI messages from the authoritative runtime.
- **Special-case runtime registration inside the runner** — couples optional UI lifecycle to the kernel instead of using the native module contract.
