# Docs index

Map of all documentation in this repo.

## Architecture

- [architecture.md](architecture.md) — engine design overview: core components, messaging, extension model, diagrams.
- [structure.md](structure.md) — static structure: components, dependency rules, source layout.

## Detailed design

- [design/messaging.md](design/messaging.md) — envelope, topics, request/reply, delivery guarantees, flow control.
- [design/extensions.md](design/extensions.md) — extension contract, execution model, lifecycle, faults, hot reload.
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
