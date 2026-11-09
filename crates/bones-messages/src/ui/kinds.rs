//! The standard widget kinds' typed properties (ADR-048). Each is a
//! `WidgetProps`, placed with `Node::widget(id, &props)` or
//! `props.node(id)`; their bytes are specified in `wit/wire-format.json`.

mod button;
mod checkbox;
mod empty_state;
mod heading;
mod hyperlink;
mod icon;
mod icon_button;
mod image;
mod label;
mod progress;
mod select;
mod selectable;
mod separator;
mod slider;
mod space;
mod spinner;
mod strong;
mod text_edit;
mod toast;
mod weak;

pub use button::Button;
pub use checkbox::Checkbox;
pub use empty_state::EmptyState;
pub use heading::Heading;
pub use hyperlink::Hyperlink;
pub use icon::Icon;
pub use icon_button::IconButton;
pub use image::{Fit, Image};
pub use label::Label;
pub use progress::Progress;
pub use select::Select;
pub use selectable::Selectable;
pub use separator::Separator;
pub use slider::Slider;
pub use space::Space;
pub use spinner::Spinner;
pub use strong::Strong;
pub use text_edit::TextEdit;
pub use toast::Toast;
pub use weak::Weak;

#[cfg(test)]
mod tests;
