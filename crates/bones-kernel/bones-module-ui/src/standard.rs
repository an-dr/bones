//! The standard widget kinds (ADR-048), each a `Kind` registered exactly
//! as an embedder registers its own.

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
mod title_bar;
mod toast;
mod weak;

use crate::registry::Registry;

pub(crate) fn register_all(registry: &mut Registry) {
    registry.register(button::ButtonKind);
    registry.register(checkbox::CheckboxKind);
    registry.register(empty_state::EmptyStateKind);
    registry.register(heading::HeadingKind);
    registry.register(hyperlink::HyperlinkKind);
    registry.register(icon::IconKind);
    registry.register(icon_button::IconButtonKind);
    registry.register(image::ImageKind);
    registry.register(label::LabelKind);
    registry.register(progress::ProgressKind);
    registry.register(select::SelectKind);
    registry.register(selectable::SelectableKind);
    registry.register(separator::SeparatorKind);
    registry.register(slider::SliderKind);
    registry.register(space::SpaceKind);
    registry.register(spinner::SpinnerKind);
    registry.register(strong::StrongKind);
    registry.register(text_edit::TextEditKind);
    registry.register(title_bar::TitleBarKind);
    registry.register(toast::ToastKind);
    registry.register(weak::WeakKind);
}
