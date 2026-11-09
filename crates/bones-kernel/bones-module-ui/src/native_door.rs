use std::collections::HashMap;

use bones_kernel_infra::ui_port::UiPort;
use bones_messages::ui::{Event, Spec};

/// The ui module's side of the `ui` service: trees native modules showed
/// this frame, in the order their owners first showed them, and the events
/// waiting for them.
pub(crate) struct NativeDoor {
    pub(crate) shown: Vec<(String, Spec)>,
    pub(crate) events: HashMap<String, Vec<Event>>,
    pub(crate) zoom: f32,
}

impl Default for NativeDoor {
    fn default() -> Self {
        Self {
            shown: Vec::new(),
            events: HashMap::new(),
            zoom: 1.0,
        }
    }
}

impl UiPort for NativeDoor {
    fn show(&mut self, owner: &str, spec: Spec) {
        upsert(&mut self.shown, owner, spec);
    }

    fn take_events(&mut self, owner: &str) -> Vec<Event> {
        self.events.remove(owner).unwrap_or_default()
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.5, 2.0);
    }
}

/// Replaces `owner`'s entry, or appends one, keeping first-shown order: the
/// order trees are drawn in, and the order the first Screen claim is
/// decided by.
pub(crate) fn upsert(shown: &mut Vec<(String, Spec)>, owner: &str, spec: Spec) {
    match shown.iter_mut().find(|(existing, _)| existing == owner) {
        Some(entry) => entry.1 = spec,
        None => shown.push((owner.to_string(), spec)),
    }
}
