use crate::Rect;

/// One button after a menu layout assigns its logical bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonLayout {
    pub id: u32,
    pub label: String,
    pub bounds: Rect,
}
