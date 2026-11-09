use std::collections::HashMap;

use bones_kernel_infra::ui_port::{Theme, UiPort};
use bones_messages::ui::{Event, Spec};

/// The ui module's side of the `ui` service: trees native modules showed
/// this frame, in the order their owners first showed them, and the events
/// waiting for them.
/// An image upload or release, applied at the start of the next frame.
pub(crate) enum ImageOp {
    Set {
        handle: u32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Free {
        handle: u32,
    },
}

pub(crate) struct NativeDoor {
    pub(crate) shown: Vec<(String, Spec)>,
    pub(crate) images: Vec<(String, ImageOp)>,
    pub(crate) events: HashMap<String, Vec<Event>>,
    pub(crate) zoom: f32,
    pub(crate) theme: Theme,
}

impl Default for NativeDoor {
    fn default() -> Self {
        Self {
            shown: Vec::new(),
            images: Vec::new(),
            events: HashMap::new(),
            zoom: 1.0,
            theme: Theme::System,
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

    fn set_image(&mut self, owner: &str, handle: u32, width: u32, height: u32, rgba: Vec<u8>) {
        let op = ImageOp::Set {
            handle,
            width,
            height,
            rgba,
        };
        self.images.push((owner.to_string(), op));
    }

    fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    fn free_image(&mut self, owner: &str, handle: u32) {
        self.images
            .push((owner.to_string(), ImageOp::Free { handle }));
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
