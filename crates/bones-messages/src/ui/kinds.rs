//! The standard widget kinds' typed properties (ADR-048). Each is a
//! `WidgetProps`, placed with `Node::widget(id, &props)` or
//! `props.node(id)`; their bytes are specified in `wit/wire-format.json`.

mod button;
mod label;
mod text_edit;

pub use button::Button;
pub use label::Label;
pub use text_edit::TextEdit;

#[cfg(test)]
mod tests;
