//! Typed `ui/*` messages (ADR-005, ADR-048, design/presentation.md): the
//! widget tree an owner describes each frame, and the events the ui module
//! reports back.
//!
//! - A `Spec` is a root (`Window` or `Screen`) over a tree of `Node`s.
//! - A `Node` is a layout container or a widget. A widget names its kind,
//!   so the wire format never changes when a kind is added (open for
//!   extension); its properties travel as bytes that kind defines.
//! - `kinds` holds the standard kinds' typed properties.

mod changed;
mod clicked;
mod event;
pub mod kinds;
mod layout;
mod node;
mod panel_side;
mod root;
mod spec;
mod widget_props;

pub use changed::Changed;
pub use clicked::Clicked;
pub use event::Event;
pub use layout::Layout;
pub use node::{Node, MAX_DEPTH};
pub use panel_side::PanelSide;
pub use root::Root;
pub use spec::Spec;
pub use widget_props::WidgetProps;

#[cfg(test)]
mod tests;
