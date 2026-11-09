//! The standard widget kinds (ADR-048), each a `Kind` registered exactly
//! as an embedder registers its own.

mod button;
mod label;
mod text_edit;

use crate::registry::Registry;

pub(crate) fn register_all(registry: &mut Registry) {
    registry.register(label::LabelKind);
    registry.register(button::ButtonKind);
    registry.register(text_edit::TextEditKind);
}
