# Architecture decisions

Every lasting architectural decision, in the order it was made. An ADR is **immutable**: it records what was decided and why, at the time, and is never edited to match what the system later became. A decision that changes is replaced by a later ADR, and this table is where that shows.

Most readers want the [architecture chapter](../architecture/index.md) instead. It states what is true now, in one pass, and cites the ADR behind each statement. These files are the audit trail underneath it — read one when you need to know *why*, or when you are about to overturn it.

## Status

| # | Decision | Status |
| ---: | --- | --- |
| [001](ADR-001-wasm-component-model.md) | WASM Component Model as the extension ABI | active |
| [002](ADR-002-engine-owned-rendering.md) | Engine-owned rendering via draw commands | active |
| [003](ADR-003-hybrid-messaging.md) | Hybrid messaging: pub/sub topics plus direct request/reply | active |
| [004](ADR-004-event-driven-execution.md) | Event-driven extension execution with optional tick | active |
| [005](ADR-005-egui-ui-layer.md) | egui UI layer in the core, widgets as bus messages | active |
| [006](ADR-006-wry-web-panels.md) | Web UI via wry web panels (optional feature) | active |
| [007](ADR-007-watchdog-quarantine.md) | Extension watchdog and quarantine | active |
| [008](ADR-008-layered-input-focus.md) | Layered input focus, top layer consumes | active — refined by [031](ADR-031-native-modules-reach-each-other-only-through-services.md) |
| [009](ADR-009-delivery-semantics.md) | Bus delivery: per-sender FIFO, at-most-once | active |
| [010](ADR-010-synchronous-send.md) | Synchronous send: request/reply completes within the call | active |
| [011](ADR-011-native-core-modules.md) | Native core modules: kernel plus consumer-composed modules | active |
| [012](ADR-012-injected-logging-sink.md) | Logging via an injected sink trait | active |
| [013](ADR-013-bus-on-pubsub-bus.md) | Bus built on pubsub-bus, persistent adapter and deferred dispatch | active — **corrected by [015](ADR-015-deferred-dispatch-remains-mandatory.md)** |
| [014](ADR-014-headless-runner-skeleton.md) | Headless runner skeleton: step-driven, injected bus, virtual clock | active |
| [015](ADR-015-deferred-dispatch-remains-mandatory.md) | Deferred dispatch remains mandatory regardless of the pubsub-bus fix | active — corrects [013](ADR-013-bus-on-pubsub-bus.md) |
| [016](ADR-016-typed-core-messages.md) | Typed core messages over the byte-oriented bus | active |
| [017](ADR-017-native-module-trait-and-typed-service-registry.md) | Native module trait and typed service registry | active |
| [018](ADR-018-core-2d-presentation-input-and-persistence-gaps.md) | Core 2D presentation, input, and persistence gaps | active |
| [019](ADR-019-2d-game-core-module-native-bought-dependencies.md) | 2D game-core module: native, bought dependencies | active |
| [020](ADR-020-persistence-is-kernel-tier-host-lifecycle-and-persistence-merge-into-wasm-extensions.md) | Persistence is kernel-tier; host, lifecycle and persistence merge | active |
| [021](ADR-021-physics-backend-abstraction-split-rapier2d-out-of-game-core-add-a-retro-backend.md) | Physics backend abstraction, split out of game-core | **superseded by [022](ADR-022-physics-stays-inside-game-core-internal-physics-tiles-graphics-grouping-no-separate-crates.md)** |
| [022](ADR-022-physics-stays-inside-game-core-internal-physics-tiles-graphics-grouping-no-separate-crates.md) | Physics stays inside game-core as internal modules | active — supersedes [021](ADR-021-physics-backend-abstraction-split-rapier2d-out-of-game-core-add-a-retro-backend.md) |
| [023](ADR-023-evolve-game-core-entity-operations-additively.md) | Evolve game-core entity operations additively | active |
| [024](ADR-024-runtime-managed-extension-activation.md) | Runtime-managed extension activation | active |
| [025](ADR-025-game-ui-is-a-theme-free-guest-toolkit.md) | Game UI is a theme-free guest toolkit | active |
| [026](ADR-026-game-core-publishes-authoritative-entity-transform-snapshots.md) | game-core publishes authoritative entity transform snapshots | active |
| [027](ADR-027-feature-gate-native-presentation.md) | Feature-gate the native presentation stack | active |
| [028](ADR-028-detachable-native-modules-and-wry-presentation.md) | Attach native modules and wry presentation to a live headless engine | active |
| [029](ADR-029-the-two-version-lines-are-the-two-public-surfaces.md) | The two version lines are the two public surfaces | active |
| [030](ADR-030-package-structure-follows-consumer-use-cases.md) | Package structure follows consumer use cases | active |
| [031](ADR-031-native-modules-reach-each-other-only-through-services.md) | Native modules reach each other only through services | active |
| [032](ADR-032-the-extension-abi-is-bones-extension-with-qualified-exports.md) | The extension ABI is `bones:extension`, with qualified exports | active |
| [033](ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md) | Self-update is a crate, and its binaries avoid installer words | active |
| [034](ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md) | OS capabilities are an optional module, splittable per capability | active |

## The two reversals

Both are worth knowing about, because in each case the reversal is the interesting part.

**021 → 022.** Physics was split out of `game-core` into three sibling crates, built that way, and moved back. The split made `physics` look like a component other modules might depend on, when nothing outside `game-core` ever used it. The behaviour it introduced — swappable backends behind a trait — was kept; only the crate boundary changed. The record survives so the next person to notice that trait boundary knows it was tried.

**013 → 015.** ADR-013 said its deferred-dispatch workaround would become unnecessary once the upstream bus was fixed. Implementing the fix proved that false: reentrant publication from inside a handler deadlocks for a reason the fix does not address, so deferred dispatch is permanent. ADR-013 is otherwise sound, and its own claim is the part 015 corrects.

## Writing one

An ADR records a decision that constrains the system's shape — a boundary, a contract, a tier, a guarantee. Tactical and tooling choices do not get one; they live in the code and its README. Number it next in sequence, and if it overturns an earlier decision, say so in its own text and update the table above.
