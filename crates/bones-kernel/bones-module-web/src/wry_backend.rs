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
        init_gtk()?;
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
        pump_gtk_events();
        self.sync_bounds()
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        self.events
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }
}

/// Initializes GTK on the runner thread with an X11 display.
///
/// - Selects the backend without changing the environment of existing threads.
/// - Rejects an embedder's incompatible GTK display or thread before wry can panic.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn init_gtk() -> Result<(), String> {
    use gtk::glib::prelude::ObjectExt;

    if gtk::is_initialized() && !gtk::is_initialized_main_thread() {
        return Err("web panels must use the thread that initialized GTK".into());
    }
    if !gtk::is_initialized() {
        gtk::gdk::set_allowed_backends("x11");
        gtk::init().map_err(|error| {
            format!(
                "initializing GTK: {error}; web panels require X11/Xwayland and GDK_BACKEND=x11"
            )
        })?;
    }
    let display = gtk::gdk::Display::default().ok_or("GTK has no default display")?;
    if display.type_().name() != "GdkX11Display" {
        return Err("web panels require an X11 GTK display; launch with GDK_BACKEND=x11".into());
    }
    Ok(())
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn init_gtk() -> Result<(), String> {
    Ok(())
}

/// Drains GLib's default main context once, non-blocking.
///
/// This process's own loop is SDL's event pump, not GTK's -- nothing else
/// ever iterates GLib. WebKitGTK depends on that loop for more than input:
/// the actual page render arrives from the web process over IPC and is
/// composited into the window by a GLib-scheduled callback, so without this
/// call every panel opens, is tracked as open, and stays a solid black
/// rectangle forever -- indistinguishable from a GPU/compositor problem
/// until you notice nothing ever unblocks it. Called from `update`, which
/// already runs once per engine frame for `sync_bounds`.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn pump_gtk_events() {
    // Bounded rather than "while true": a source that keeps re-arming
    // itself (a busy socket, a fast repeating timer) would otherwise make
    // a single non-blocking call run for as long as work keeps arriving,
    // which defeats the "drain what's pending, then return" intent this
    // call exists for.
    const MAX_ITERATIONS: u32 = 64;
    let context = gtk::glib::MainContext::default();
    for _ in 0..MAX_ITERATIONS {
        if !context.iteration(false) {
            break;
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn pump_gtk_events() {}

fn panel_bounds((width, height): (u32, u32)) -> Rect {
    Rect {
        position: PhysicalPosition::new(0, 0).into(),
        size: PhysicalSize::new(width, height).into(),
    }
}
