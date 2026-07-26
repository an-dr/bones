use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bones_messages::web::PanelSource;
use raw_window_handle::HasWindowHandle;
use send_wrapper::SendWrapper;
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebView, WebViewBuilder};

use crate::{Backend, BackendEvent};

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
    bounds: Rect,
    panels: HashMap<PanelKey, SendWrapper<WebView>>,
    events: EventQueue,
}

impl WryBackend {
    /// Captures the SDL window's native handle and initial client size.
    ///
    /// The SDL window must outlive this backend. The runner satisfies that
    /// invariant by shutting modules down before dropping the renderer/window.
    pub fn new(window: &sdl3::video::Window) -> Result<Self, String> {
        let raw = window
            .window_handle()
            .map_err(|error| format!("reading SDL window handle: {error}"))?
            .as_raw();
        let (width, height) = window.size();
        Ok(Self {
            parent: SendWrapper::new(ParentHandle::new(raw)),
            bounds: Rect {
                position: LogicalPosition::new(0, 0).into(),
                size: LogicalSize::new(width, height).into(),
            },
            panels: HashMap::new(),
            events: Arc::new(Mutex::new(Vec::new())),
        })
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
            .with_bounds(self.bounds)
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
            PanelSource::Html(html) => builder.with_html(html),
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

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        self.events
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }
}
