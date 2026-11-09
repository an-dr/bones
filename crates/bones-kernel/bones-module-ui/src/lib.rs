//! egui UI module (ADR-005, design/presentation.md): decodes `ui/spec`
//! messages into an embedded `egui::Context`, publishes `ui/clicked` and
//! `ui/changed` back, and submits the tessellated output through the
//! `draw-target` service (design/modules.md) it consumes at `init`.
//!
//! This crate does not depend on `bones-module-renderer` and never names
//! it: any module providing a surface can back it, which is what makes
//! `renderer` replaceable without touching anything here.
//!
//! TODO: `ui/clicked`/`ui/changed` are broadcast on shared topics, not
//! targeted to the owning extension only (presentation.md's stated
//! contract) — every extension subscribed to `ui/*` sees every event and
//! must filter by its own widget ids. Fine while one extension uses `ui/*`
//! at a time; revisit (direct send, or a per-extension topic) once that
//! stops holding.

mod chrome;
mod display_scale;
mod draw_context;
mod fonts;
mod input_translation;
mod kind;
mod look;
mod native_door;
mod output_translation;
mod platform_output;
mod registry;
mod standard;
mod tree;
mod ui;

pub use bones_messages::ui::Event;
pub use draw_context::DrawContext;
pub use egui;
pub use kind::Kind;
pub use ui::Ui;
