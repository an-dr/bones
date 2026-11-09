//! The `ui` service (ADR-048, ADR-031): the native door into the ui
//! module. A native module hands its widget tree over here, without the
//! bus, and takes back the events its widgets produced.
//!
//! - It lives in this always-present tier for the reason `draw-target`
//!   does: the contract sits between two optional modules, so neither can
//!   own it without the consumer depending on the provider's crate.
//! - `UiView` and `ViewModule` are the convenient way to use it: describe
//!   the screen from state, handle events, and let `ViewModule` do the
//!   per-frame calls.

use std::sync::{Arc, Mutex};

use bones_messages::ui::{Event, Spec};

use crate::bus::{Envelope, Handler, Module, ModuleContext};

/// Light or dark, as an application's theme setting chooses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the desktop's light or dark mode (the default).
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// What the ui module offers a native module.
pub trait UiPort: Send {
    /// Shows `owner`'s tree this frame, replacing anything it showed
    /// earlier in the same frame. A tree not shown again next frame is not
    /// drawn next frame.
    fn show(&mut self, owner: &str, spec: Spec);

    /// The events `owner`'s widgets produced since the last call, oldest
    /// first.
    fn take_events(&mut self, owner: &str) -> Vec<Event>;

    /// Scales the whole interface by `zoom` on top of the display's own
    /// scale, for an application's interface-size setting. Clamped to
    /// 0.5–2; 1 is the display's size.
    fn set_zoom(&mut self, zoom: f32);

    /// Uploads an image `owner`'s `image` widgets show by `handle`:
    /// `width * height` straight RGBA8 pixels, row by row. A second upload
    /// under the same handle replaces the first.
    fn set_image(&mut self, owner: &str, handle: u32, width: u32, height: u32, rgba: Vec<u8>);

    /// Forgets an image `owner` uploaded.
    fn free_image(&mut self, owner: &str, handle: u32);

    /// Chooses light, dark, or the desktop's mode for the native look; has
    /// no effect when the look is turned off.
    fn set_theme(&mut self, theme: Theme);
}

/// The service's registry key: shared, because several native modules may
/// each show their own tree. Resolve it with `ModuleContext::get_service`
/// and clone it; consuming it would take it from the others.
pub type UiService = Arc<Mutex<dyn UiPort>>;

/// A native interface described from state, as a `ViewModule` drives it.
pub trait UiView: Send + 'static {
    /// Called once at build time, after the ui module is up: subscribe to
    /// topics or resolve services here.
    fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
        Ok(())
    }

    /// One event from this view's widgets, before the next `view`.
    fn on_event(&mut self, event: Event);

    /// A bus message for this view's endpoint name.
    fn on_message(&mut self, _envelope: &Envelope) {}

    /// The whole interface for this frame.
    fn view(&mut self) -> Spec;
}

/// Runs a `UiView` as a native module named `name`: each frame it hands
/// the view its events, then shows the tree the view returns.
pub struct ViewModule<V: UiView> {
    name: String,
    view: V,
    port: Option<UiService>,
}

impl<V: UiView> ViewModule<V> {
    /// The module registers under `name`, which is also the tree's owner.
    pub fn new(name: impl Into<String>, view: V) -> Self {
        Self {
            name: name.into(),
            view,
            port: None,
        }
    }
}

impl<V: UiView> Handler for ViewModule<V> {
    fn handle(&mut self, envelope: &Envelope) {
        self.view.on_message(envelope);
    }
}

impl<V: UiView> Module for ViewModule<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        let port = ctx.get_service::<UiService>().cloned().ok_or_else(|| {
            format!(
                "'{}' needs the ui service; compose .ui() before this module",
                self.name
            )
        })?;
        self.port = Some(port);
        self.view.init(ctx)
    }

    /// After the tick, so the tree reflects everything this frame's
    /// messages changed, and before `render`, where the ui module draws.
    fn after_tick(&mut self) {
        let Some(port) = &self.port else { return };
        let events = port.lock().unwrap().take_events(&self.name);
        for event in events {
            self.view.on_event(event);
        }
        let spec = self.view.view();
        port.lock().unwrap().show(&self.name, spec);
    }
}

#[cfg(test)]
mod tests;
