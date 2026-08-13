//! egui UI module (ADR-005, design/presentation.md): decodes `ui/spec`
//! messages into an embedded `egui::Context`, publishes `ui/clicked` and
//! `ui/changed` back, and submits the tessellated output to the renderer
//! directly — design/modules.md's `draw-target` service, direct-wired for
//! now the same way `renderer` itself is wired into `Engine` rather than
//! through a generic module trait (see docs/structure.md).
//!
//! TODO: `ui/clicked`/`ui/changed` are broadcast on shared topics, not
//! targeted to the owning extension only (presentation.md's stated
//! contract) — every extension subscribed to `ui/*` sees every event and
//! must filter by its own widget ids. Fine while one extension uses `ui/*`
//! at a time; revisit (direct send, or a per-extension topic) once that
//! stops holding.

mod input_translation;
mod output_translation;
mod owned_widget;
mod pending_spec;
mod ui;

pub use ui::Ui;
