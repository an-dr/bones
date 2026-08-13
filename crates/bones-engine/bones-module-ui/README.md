# ui

egui integration (ADR-005, design/presentation.md): decodes `ui/spec` messages into an embedded `egui::Context`, publishes `ui/clicked` and `ui/changed` back, and submits the tessellated output to `renderer` directly — design/modules.md's `draw-target` service, direct-wired for now the same way `renderer` itself is wired into `Engine` rather than through a generic module trait (see docs/structure.md).

- `ui/spec` — a full per-frame panel; immediate-mode (ADR-005), so not republishing this frame draws nothing. Small vocabulary: `Label`, `TextEdit`, `Button`.
- `ui/clicked` / `ui/changed` — published back when a `Button` is clicked or a `TextEdit`'s text changes. Broadcast on shared topics, not targeted to the owning extension — every extension subscribed to `ui/*` sees every event and must filter by its own widget ids (fine while one extension uses `ui/*` at a time; see the crate's own TODO).

Raw SDL input is translated into egui input via `Ui::feed_event`, which returns whether this layer claims the event (ADR-008: top layer consumes) — the platform layer uses this to decide whether the event still reaches `input/*`.
