use bones_messages::ui::Widget;

/// Owned counterpart to `bones_messages::ui::Widget`'s borrowed variants —
/// `PendingSpec` needs to retain a spec past the envelope payload's lifetime,
/// so each field is copied into an owned `String` here.
pub(crate) enum OwnedWidget {
    Label(String),
    TextEdit { id: u32, text: String },
    Button { id: u32, label: String },
}

impl From<&Widget<'_>> for OwnedWidget {
    fn from(widget: &Widget<'_>) -> Self {
        match widget {
            Widget::Label { text } => OwnedWidget::Label((*text).to_string()),
            Widget::TextEdit { id, text } => OwnedWidget::TextEdit {
                id: *id,
                text: (*text).to_string(),
            },
            Widget::Button { id, label } => OwnedWidget::Button {
                id: *id,
                label: (*label).to_string(),
            },
        }
    }
}
