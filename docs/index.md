# Docs index

Map of all documentation in this repo.

## Documentation policy

Docs capture **behavior and boundaries, not code**. Each layer has an
altitude, and staying above code details is deliberate:

- [architecture.md](architecture.md) — system altitude: components, message
  flows, lifecycles.
- `design/` — one level more detailed than architecture: behavior contracts
  (semantics, guarantees, state machines) and logical structure. Never exact
  signatures, function names, or file-level layout.
- `adr/` — immutable decisions with rationale; superseded by new ADRs, never
  edited.
- [roadmap.md](roadmap.md) — remaining work only; completed increments are
  deleted, not checked off.

The altitude test: **an average refactoring must not require a documentation
update.** Docs change when observable behavior, a contract, or a component
boundary changes — not when code moves, splits, or gets renamed. If a doc
keeps needing updates during refactorings, it is written too low; raise its
altitude instead of maintaining it.

## Architecture

- [architecture.md](architecture.md) — engine design overview: core components, messaging, extension model, diagrams.
- [structure.md](structure.md) — static structure: components, dependency rules, source layout.
- [roadmap.md](roadmap.md) — remaining implementation increments with demo milestones.

## Detailed design

- [design/messaging.md](design/messaging.md) — envelope, topics, request/reply, delivery guarantees, flow control.
- [design/extensions.md](design/extensions.md) — extension contract, execution model, lifecycle, faults, hot reload.
- [design/modules.md](design/modules.md) — native modules, frame phases, service traits, composition root, embedding.
- [design/presentation.md](design/presentation.md) — gfx/ui/web backends, input and focus routing.
- [design/platform.md](design/platform.md) — window, tray, input, frame loop, shutdown.

## Examples

- [examples/egui-app.md](examples/egui-app.md) — worked example: "notes", a widget-UI extension.
- [examples/web-app.md](examples/web-app.md) — worked example: "dashboard", a web-panel extension.

## Decisions

Immutable ADRs in [adr/](adr/):

- [ADR-001](adr/ADR-001-wasm-component-model.md) — WASM Component Model as the extension ABI
- [ADR-002](adr/ADR-002-engine-owned-rendering.md) — engine-owned rendering via draw commands
- [ADR-003](adr/ADR-003-hybrid-messaging.md) — hybrid messaging: pub/sub topics + direct request/reply
- [ADR-004](adr/ADR-004-event-driven-execution.md) — event-driven extension execution with optional tick
- [ADR-005](adr/ADR-005-egui-ui-layer.md) — egui UI layer in the core, widgets as bus messages
- [ADR-006](adr/ADR-006-wry-web-panels.md) — web UI via wry web panels (optional feature)
- [ADR-007](adr/ADR-007-watchdog-quarantine.md) — extension watchdog and quarantine
- [ADR-008](adr/ADR-008-layered-input-focus.md) — layered input focus, top layer consumes
- [ADR-009](adr/ADR-009-delivery-semantics.md) — bus delivery: per-sender FIFO, at-most-once
- [ADR-010](adr/ADR-010-synchronous-send.md) — synchronous send: request/reply completes within the call
- [ADR-011](adr/ADR-011-native-core-modules.md) — native core modules: kernel plus consumer-composed modules
- [ADR-012](adr/ADR-012-injected-logging-sink.md) — logging via an injected sink trait, resolved at kernel construction
- [ADR-013](adr/ADR-013-bus-on-pubsub-bus.md) — bus built on pubsub-bus, with a persistent adapter and deferred dispatch
- [ADR-014](adr/ADR-014-headless-runner-skeleton.md) — headless runner skeleton: step-driven, injected bus, virtual clock
