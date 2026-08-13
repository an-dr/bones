use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bones_messages::web::PanelSource;
use send_wrapper::SendWrapper;
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::{Rect, WebView, WebViewBuilder};

use crate::{Backend, BackendEvent};

mod page_source;
mod parent_handle;
use parent_handle::ParentHandle;

#[cfg(test)]
mod tests;

type PanelKey = (String, String);
type EventQueue = Arc<Mutex<Vec<BackendEvent>>>;

/// Wry child-webview backend attached to an SDL window.
///
/// Construct and use this backend on the runner thread. `SendWrapper` makes
/// the backend compatible with the module boundary while enforcing wry's
/// native thread affinity whenever a view is accessed or dropped.
pub struct WryBackend {
    parent: SendWrapper<ParentHandle>,
    pixel_size: (u32, u32),
    panels: HashMap<PanelKey, SendWrapper<WebView>>,
    events: EventQueue,
}

impl WryBackend {
    /// Shares the SDL window and captures its initial client size.
    pub fn new(window: &sdl3::video::Window) -> Result<Self, String> {
        Ok(Self {
            parent: SendWrapper::new(ParentHandle::new(window)),
            pixel_size: window.size_in_pixels(),
            panels: HashMap::new(),
            events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn sync_bounds(&mut self) -> Result<(), String> {
        let pixel_size = self.parent.pixel_size();
        if pixel_size == self.pixel_size {
            return Ok(());
        }

        let bounds = panel_bounds(pixel_size);
        for ((owner, panel), view) in &mut self.panels {
            view.set_bounds(bounds)
                .map_err(|error| format!("resizing panel '{owner}/{panel}': {error}"))?;
        }
        self.pixel_size = pixel_size;
        Ok(())
    }

    fn panel_mut(&mut self, owner: &str, panel: &str) -> Result<&mut WebView, String> {
        self.panels
            .get_mut(&(owner.to_string(), panel.to_string()))
            .map(|view| &mut **view)
            .ok_or_else(|| format!("unknown panel '{owner}/{panel}'"))
    }
}

impl Backend for WryBackend {
    fn open(&mut self, owner: &str, panel: &str, source: PanelSource<'_>) -> Result<(), String> {
        let key = (owner.to_string(), panel.to_string());
        if self.panels.contains_key(&key) {
            return Err(format!("panel '{owner}/{panel}' already exists"));
        }

        let event_owner = owner.to_string();
        let event_panel = panel.to_string();
        let events = Arc::clone(&self.events);
        let builder = WebViewBuilder::new()
            .with_bounds(panel_bounds(self.pixel_size))
            .with_ipc_handler(move |request| {
                if let Ok(mut events) = events.lock() {
                    events.push(BackendEvent::PageMessage {
                        owner: event_owner.clone(),
                        panel: event_panel.clone(),
                        json: request.body().clone(),
                    });
                }
            });
        let builder = match source {
            // Served rather than set as a string, so the page gets a real
            // origin — see `page_source` for what an opaque one costs.
            PanelSource::Html(html) => {
                let page = html.as_bytes().to_vec();
                builder
                    .with_custom_protocol(
                        page_source::PROTOCOL.to_string(),
                        move |_id, _request| page_source::response(&page),
                    )
                    .with_url(page_source::URL)
            }
            PanelSource::Url(url) => builder.with_url(url),
        };
        let view = builder
            .build_as_child(&*self.parent)
            .map_err(|error| format!("opening panel '{owner}/{panel}': {error}"))?;
        self.panels.insert(key, SendWrapper::new(view));
        Ok(())
    }

    fn close(&mut self, owner: &str, panel: &str) -> Result<(), String> {
        self.panels
            .remove(&(owner.to_string(), panel.to_string()))
            .map(drop)
            .ok_or_else(|| format!("unknown panel '{owner}/{panel}'"))
    }

    fn navigate(&mut self, owner: &str, panel: &str, url: &str) -> Result<(), String> {
        self.panel_mut(owner, panel)?
            .load_url(url)
            .map_err(|error| format!("navigating panel '{owner}/{panel}': {error}"))
    }

    fn send_json(&mut self, owner: &str, panel: &str, json: &str) -> Result<(), String> {
        let json = serde_json::to_string(json)
            .map_err(|error| format!("encoding message for '{owner}/{panel}': {error}"))?;
        let script = format!(
            "window.dispatchEvent(new CustomEvent('bones-message', {{ detail: {json} }}));"
        );
        self.panel_mut(owner, panel)?
            .evaluate_script(&script)
            .map_err(|error| format!("sending message to panel '{owner}/{panel}': {error}"))
    }

    fn update(&mut self) -> Result<(), String> {
        self.sync_bounds()
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        self.events
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }
}

fn panel_bounds((width, height): (u32, u32)) -> Rect {
    Rect {
        position: PhysicalPosition::new(0, 0).into(),
        size: PhysicalSize::new(width, height).into(),
    }
}
