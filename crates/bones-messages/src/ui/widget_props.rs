use crate::DecodeError;

use super::Node;

/// The typed properties of one widget kind, and how they travel as the
/// bytes of a `Node::Widget`.
///
/// - Implement it for a custom kind to place that kind with
///   `Node::widget`, exactly as the standard kinds in `ui::kinds` do.
/// - `KIND` is the registry key; choose a name no other kind uses.
pub trait WidgetProps: Sized {
    /// The kind name the ui module's registry looks up.
    const KIND: &'static str;

    /// The properties as the bytes a `Node::Widget` carries.
    fn encode(&self) -> Vec<u8>;

    /// Reads properties written by `encode`.
    fn decode(props: &[u8]) -> Result<Self, DecodeError>;

    /// A widget node of this kind with this id.
    fn node(&self, id: u32) -> Node {
        Node::widget(id, self)
    }
}
