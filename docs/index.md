# Docs index

Map of all documentation in this repo.

- [architecture.md](architecture.md) — engine design: core components, messaging, extension model, diagrams.
- [adr/](adr/) — immutable architecture decision records:
  - [ADR-001](adr/ADR-001-wasm-component-model.md) — WASM Component Model as the extension ABI
  - [ADR-002](adr/ADR-002-engine-owned-rendering.md) — engine-owned rendering via draw commands
  - [ADR-003](adr/ADR-003-hybrid-messaging.md) — hybrid messaging: pub/sub topics + direct request/reply
  - [ADR-004](adr/ADR-004-event-driven-execution.md) — event-driven extension execution with optional tick
  - [ADR-005](adr/ADR-005-egui-ui-layer.md) — egui UI layer in the core, widgets as bus messages
  - [ADR-006](adr/ADR-006-wry-web-panels.md) — web UI via wry web panels (optional feature)
