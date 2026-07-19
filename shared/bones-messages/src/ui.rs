//! Typed `ui/*` widget-spec and interaction messages (ADR-005, design/
//! presentation.md): the immediate-mode vocabulary extensions use to
//! describe egui panels, and the events the ui module publishes back. The
//! vocabulary is deliberately small at first (`Label`, `TextEdit`,
//! `Button`) — enough for the "notes" worked example; grows as a versioned
//! addition, same as `gfx`'s command set.

mod changed;
mod clicked;
mod spec;
mod widget;

pub use changed::Changed;
pub use clicked::Clicked;
pub use spec::Spec;
pub use widget::Widget;

#[cfg(test)]
mod tests;
