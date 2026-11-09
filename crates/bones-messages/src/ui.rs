//! Typed `ui/*` messages (ADR-005, ADR-048, design/presentation.md): the
//! widget tree an owner describes each frame, and the events the ui module
//! reports back.
//!
//! - A `Spec` is a root (`Window` or `Screen`) over a tree of `Node`s.
//! - A `Node` is a layout container or a widget. A widget names its kind,
//!   so the wire format never changes when a kind is added (open for
//!   extension); its properties travel as bytes that kind defines.
//! - `kinds` holds the standard kinds' typed properties.
//! - `icons` names every bundled Lucide icon (ADR-049).

mod appeared;
mod changed;
mod clicked;
mod dropped;
mod event;
mod free_image;
pub mod icons;
mod key_pressed;
pub mod kinds;
mod layout;
mod node;
mod panel_side;
mod reached_end;
mod root;
mod selected;
mod set_image;
mod spec;
mod submitted;
mod toggled;
mod value_changed;
mod widget_props;

pub use appeared::Appeared;
pub use changed::Changed;
pub use clicked::Clicked;
pub use dropped::Dropped;
pub use event::Event;
pub use free_image::FreeImage;
pub use key_pressed::KeyPressed;
pub use layout::Layout;
pub use node::{Node, MAX_DEPTH};
pub use panel_side::PanelSide;
pub use reached_end::ReachedEnd;
pub use root::Root;
pub use selected::Selected;
pub use set_image::SetImage;
pub use spec::Spec;
pub use submitted::Submitted;
pub use toggled::Toggled;
pub use value_changed::ValueChanged;
pub use widget_props::WidgetProps;

#[cfg(test)]
mod tests;
