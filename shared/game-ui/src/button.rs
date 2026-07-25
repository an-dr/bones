/// Caller-owned identity and label for one menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub id: u32,
    pub label: String,
}

impl Button {
    pub fn new(id: u32, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}
