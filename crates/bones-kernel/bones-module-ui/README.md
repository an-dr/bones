# ui

egui integration ([ADR-005](../../../docs/adr/ADR-005-egui-ui-layer.md), [ADR-048](../../../docs/adr/ADR-048-one-widget-tree-for-native-modules-and-extensions.md)): draws each owner's widget tree in an embedded egui context, reports interaction events back, and submits the tessellated output to `renderer` through the `draw-target` service.

Every widget is drawn through a registry of kinds. `src/standard/` holds the standard ones; an embedder adds its own by implementing `Kind` and calling `Ui::register`, and the re-exported `egui` is the version to draw with.

- Immediate-mode behaviour, the widget vocabulary, and input claiming: [docs/design/presentation.md](../../../docs/design/presentation.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

The service wiring and frame phases are in [docs/design/modules.md](../../../docs/design/modules.md).
