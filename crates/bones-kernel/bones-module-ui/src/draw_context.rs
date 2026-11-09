use std::collections::HashMap;

use bones_messages::ui::Event;

/// What a `Kind` may reach while drawing one widget: the owner's events,
/// and the images the owner uploaded.
pub struct DrawContext<'a> {
    events: &'a mut Vec<Event>,
    images: Option<&'a HashMap<u32, egui::TextureHandle>>,
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(
        events: &'a mut Vec<Event>,
        images: Option<&'a HashMap<u32, egui::TextureHandle>>,
    ) -> Self {
        Self { events, images }
    }

    /// Reports what the user did to this widget, for its owner.
    pub fn emit(&mut self, event: Event) {
        self.events.push(event);
    }

    /// The owner's image uploaded under `handle`, if there is one.
    pub fn image(&self, handle: u32) -> Option<&egui::TextureHandle> {
        self.images?.get(&handle)
    }
}
