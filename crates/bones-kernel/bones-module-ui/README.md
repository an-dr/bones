# ui

egui integration ([ADR-005](../../../docs/adr/ADR-005-egui-ui-layer.md)): decodes `ui/*` widget specs into an embedded egui context, publishes interaction events back, and submits the tessellated output to `renderer` through the `draw-target` service.

- Immediate-mode behaviour, the widget vocabulary, and input claiming: [docs/design/presentation.md](../../../docs/design/presentation.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

The service wiring and frame phases are in [docs/design/modules.md](../../../docs/design/modules.md).
