# ADR-018: Core 2D presentation, input, and persistence gaps

## Problem

Building any content-rich 2D consumer of bones — a game, but equally a map viewer or a media-heavy GUI app — needs capabilities `gfx/*` and `input/*` don't have yet, and one bones has never had: state that survives extension reload. None of these are specific to games:

- `gfx/*` has only `Clear`, `LoadSprite`, `DrawSprite` — no camera/viewport transform, no per-batch layer compositing (documented in [design/presentation.md](../design/presentation.md), never implemented), no rotate/scale/flip/tint, no shapes or text (also documented, not built).
- `input/*` has only `KeyDown`/`KeyUp` — mouse and controller are named as platform responsibilities in [design/platform.md](../design/platform.md) but not implemented.
- There is no audio module at all.
- WASM extensions have no OS API access (by design — the sandbox), so there is currently no mechanism by which an extension could persist state across a reload. [design/extensions.md](../design/extensions.md) names this an open question.

## Decision

Treat all of the above as core-engine scope, not something deferred into a future game module — implemented as extensions of the contracts and module pattern already committed in ADR-002, ADR-007, ADR-008, and ADR-011:

- **Camera/viewport** — owned by the renderer, extends `gfx/*`. Extensions submit world-space draw commands; the renderer applies the active transform, the same way it already owns compositing.
- **Layering, sprite transforms, shapes, text** — extend the `gfx/*` `Command` enum. Implementation-increment work, not a new contract shape.
- **Mouse, controller** — extend `input/*`, following the existing `KeyDown`/`KeyUp` pattern.
- **Audio** — a new native module (`audio/*` vocabulary), same trust tier as `renderer`/`ui`, wrapping a bought engine-agnostic backend rather than a bones-owned mixer.
- **Persistence** — a new native module that owns file I/O; extensions request save/load over the bus instead of touching the filesystem directly. This is the sharpest gap: it is new capability crossing the extension trust boundary, not an extension of an existing one.

## Rationale

- None of these are RPG- or even game-specific — a GUI app (bones' other named use case, per README) can equally want a scrollable viewport, drawn text, mouse input, or state that survives a reload. Scoping them as core keeps the game-core module (ADR-019) free of infrastructure that isn't actually about games.
- Camera, layering, shapes, and text are additive to a contract ADR-002 already committed to (engine-owned rendering via draw commands) — no new ownership question, just unfinished vocabulary.
- Audio and persistence are new modules, but the *pattern* — a trusted native module wrapping an OS-facing concern, reached over the bus — is exactly what ADR-011 anticipated ("audio... later design rounds"). Persistence has no such precedent for the *mechanism* (extensions have never had any form of durable state), which is why it's called out explicitly rather than assumed to fall out of ADR-011 by analogy.

## Rejected alternatives

- **Fold these into the game-core module (ADR-019)** — would tie general presentation/input/persistence capability to whether a project happens to be a game, and would make the game-core module depend on things that aren't game logic.
- **Persistence via a WASI capability grant directly to extensions** — would punch a hole in the sandbox per-extension rather than mediating through a trusted module and the bus, weakening the trust boundary ADR-007's watchdog/quarantine model relies on.
