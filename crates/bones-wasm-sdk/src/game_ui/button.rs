/// Caller-owned identity and label for one menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// Caller-assigned id, returned when the button is selected or hovered.
    pub id: u32,
    /// The text drawn on the button.
    pub label: String,
}

impl Button {
    /// A button with this id and label.
    pub fn new(id: u32, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}
