use bones_messages::ui::Spec;

use crate::owned_widget::OwnedWidget;

pub(crate) struct PendingSpec {
    pub(crate) title: String,
    pub(crate) widgets: Vec<OwnedWidget>,
}

impl PendingSpec {
    pub(crate) fn from_message(spec: &Spec<'_>) -> Self {
        Self {
            title: spec.title.to_string(),
            widgets: spec.widgets.iter().map(OwnedWidget::from).collect(),
        }
    }
}
