use ab_glyph::FontRef;
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel_infra::draw_target::DrawTargetService;
use bones_kernel_infra::logging::Logger;
use bones_messages::gfx::Command;
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::renderer::{Capture, Captured, DisplayChanged, LogicalCanvas, Scope};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use sdl3::video::Window;
use send_wrapper::SendWrapper;

use crate::frame_capture::capture_png;
use crate::inner::Inner;
use crate::shared_inner::SharedInner;
use crate::window_capture::{capture_window, WindowImage};

/// Every `gfx/*` command; requested in `init` rather than by the builder,
/// so what this module listens to travels with the module.
const GFX_TOPICS: &str = "gfx/*";

/// SDL's `Window`/`Canvas` aren't `Send`/`Sync` (real thread-affinity
/// constraints on some platforms), but the vendored `pubsub-bus` crate
/// requires both on anything registered as a bus endpoint. `SendWrapper`
/// makes the type check pass while keeping the guarantee real: it panics
/// (not silent UB) if `State` is ever actually touched from a thread other
/// than the one that created it — true today, since dispatch is single-
/// threaded, but enforced rather than merely assumed.
pub struct Renderer(SendWrapper<State>);

// `State` stays in this file rather than splitting further: it's purely
// `Renderer`'s own internal store (the `SendWrapper` payload), never
// constructed or named outside this file, never meaningful on its own.
struct State {
    logger: Logger,
    // Taken directly rather than through `ServiceRegistry::consume` (design/
    // modules.md's single-consumer semantics): `Bus` is cheap to clone (an
    // `Arc` internally, `Kernel::build`'s own reasoning for handing it to
    // `.module(...)`-injected modules too), the same way `Ui::new` already
    // takes its own clone directly instead of competing for the registry's
    // one instance. Lets `renderer/display-changed` publish below.
    bus: Bus,
    // `None` until `Module::init` consumes `window-surface` and builds the
    // real SDL state — every other method panics if called first (a
    // caller/wiring bug, not a runtime condition to recover from).
    //
    // Shared rather than owned outright: `init` hands a second handle to
    // the same state out as the `draw-target` service, so a module drawing
    // above this one never needs to name `Renderer` (see `shared_inner`).
    inner: Option<SharedInner>,
    /// `renderer/capture` requests waiting for the next `present`.
    pending_captures: Vec<Capture>,
}

impl State {
    fn inner_mut(&self) -> std::sync::MutexGuard<'_, send_wrapper::SendWrapper<Inner>> {
        self.inner
            .as_ref()
            .expect("Renderer used before Module::init built its SDL state")
            .lock()
    }
}

impl Renderer {
    pub fn new(bus: Bus, logger: Logger) -> Self {
        Self(SendWrapper::new(State {
            logger,
            bus,
            inner: None,
            pending_captures: Vec::new(),
        }))
    }

    /// Shows the frame, answering every pending capture around it: the
    /// canvas is read before the flip, while it still holds the frame, and
    /// the window after, once the frame is what the window shows.
    pub fn present(&mut self) {
        let pending = std::mem::take(&mut self.0.pending_captures);
        // Read for a window capture too: it is the answer when the window
        // cannot be captured, and after the flip it is gone.
        let canvas = match pending.is_empty() {
            true => None,
            false => Some(self.image(capture_png(&self.0.inner_mut().canvas))),
        };
        self.0.inner_mut().canvas.present();
        let window = pending
            .iter()
            .any(|capture| capture.scope == Scope::Window)
            .then(|| capture_window(self.0.inner_mut().canvas.window()));
        let window = window.and_then(|captured| match captured {
            Ok(WindowImage::Read(width, height, png)) => Some((width, height, png)),
            Ok(WindowImage::SameAsCanvas) => canvas.clone(),
            Err(reason) => {
                self.0.logger.info(
                    "renderer",
                    &format!("window capture unavailable ({reason}); answering with the canvas"),
                );
                None
            }
        });
        for capture in pending {
            let (scope, (width, height, png)) = match (capture.scope, &window) {
                (Scope::Window, Some(image)) => (Scope::Window, image),
                _ => (
                    Scope::Canvas,
                    canvas.as_ref().expect("read when anything was pending"),
                ),
            };
            self.0.bus.publish(Envelope {
                topic: Captured::TOPIC.to_string(),
                sender: "renderer".to_string(),
                correlation: None,
                payload: Captured {
                    request_id: capture.request_id,
                    scope,
                    width: *width,
                    height: *height,
                    png,
                }
                .encode(),
            });
        }
    }

    /// A captured image, or an empty one with the failure logged, so a
    /// caller waiting on its `request_id` is never left without a reply.
    fn image(&self, captured: Result<(u32, u32, Vec<u8>), String>) -> (u32, u32, Vec<u8>) {
        captured.unwrap_or_else(|error| {
            self.0
                .logger
                .error("renderer", &format!("capture: {error}"));
            (0, 0, Vec::new())
        })
    }

    fn publish_logical_canvas(&mut self) {
        let (width, height) = self.0.inner_mut().logical_size();
        self.0.bus.publish(Envelope {
            topic: LogicalCanvas::TOPIC.to_string(),
            sender: "renderer".to_string(),
            correlation: None,
            payload: LogicalCanvas { width, height }.encode(),
        });
    }
}

impl Handler for Renderer {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic == Capture::TOPIC {
            match Capture::decode(&envelope.payload) {
                Ok(capture) => self.0.pending_captures.push(capture),
                Err(err) => self.0.logger.error(
                    "renderer",
                    &format!("{} from '{}': {err}", envelope.topic, envelope.sender),
                ),
            }
            return;
        }
        if envelope.topic == LifecycleEvent::TOPIC {
            if envelope.sender != "engine" {
                return;
            }
            match LifecycleEvent::decode(&envelope.payload) {
                Ok(event) if announces_logical_canvas(event.event) => {
                    self.publish_logical_canvas();
                }
                Ok(_) => {}
                Err(err) => self.0.logger.error(
                    "renderer",
                    &format!("{} from 'engine': {err}", envelope.topic),
                ),
            }
            return;
        }

        let result = match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => {
                // The requested size/fullscreen isn't necessarily what the
                // OS actually applies (fullscreen especially) - published
                // after `record` so a caller learns the real outcome
                // instead of assuming its own request took effect.
                let is_set_display = matches!(command, Command::SetDisplay(_));
                let result = self.0.inner_mut().record(&envelope.sender, command);
                if is_set_display && result.is_ok() {
                    let (width, height) = self.0.inner_mut().canvas.window().size();
                    self.0.bus.publish(Envelope {
                        topic: DisplayChanged::TOPIC.to_string(),
                        sender: "renderer".to_string(),
                        correlation: None,
                        payload: DisplayChanged { width, height }.encode(),
                    });
                }
                result
            }
            Ok(None) => {
                self.0.logger.warn(
                    "renderer",
                    &format!(
                        "unknown command '{}' from '{}'",
                        envelope.topic, envelope.sender
                    ),
                );
                return;
            }
            Err(err) => Err(err.to_string()),
        };
        if let Err(err) = result {
            self.0.logger.error(
                "renderer",
                &format!("{} from '{}': {err}", envelope.topic, envelope.sender),
            );
        }
    }
}

fn announces_logical_canvas(event: Event) -> bool {
    matches!(event, Event::Loaded | Event::Reloaded)
}

#[cfg(test)]
mod tests;

impl Module for Renderer {
    fn name(&self) -> &str {
        "renderer"
    }

    /// Consumes the `window-surface` service (design/modules.md), builds
    /// the real SDL canvas/texture-creator state, and provides that state
    /// back as the `draw-target` service; errors if no window was provided
    /// (e.g. `.renderer()` without `.window(...)`).
    ///
    /// Providing `draw-target` here, rather than the builder wiring the two
    /// modules together, is what keeps the pairing a runtime contract: any
    /// module offering the same service can stand in, and any module
    /// needing a surface consumes it without naming this crate.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        let window = ctx.consume_service::<Window>().ok_or_else(|| {
            "renderer needs a window-surface service (configure .window(...))".to_string()
        })?;
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();

        let font = FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR)
            .map_err(|e| e.to_string())?;

        let shared = SharedInner::new(Inner::new(canvas, texture_creator, font));
        ctx.provide_service::<DrawTargetService>(Box::new(shared.clone()))?;
        self.0.inner = Some(shared);

        ctx.subscribe(GFX_TOPICS);
        ctx.subscribe(LifecycleEvent::TOPIC);
        ctx.subscribe(Capture::TOPIC);
        Ok(())
    }

    /// Composites this frame's retained `gfx/*` batches (clear, then every
    /// sender's most recent draw batch) before `ui` draws above them and
    /// `present` runs (design/modules.md's `render` phase).
    fn render(&mut self) {
        if let Err(err) = self.0.inner_mut().composite() {
            self.0
                .logger
                .error("renderer", &format!("composite: {err}"));
        }
    }

    fn present(&mut self) {
        Renderer::present(self);
    }
}
