use ab_glyph::FontRef;
use bones_messages::gfx::Command;
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::renderer::{DisplayChanged, LogicalCanvas};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel::draw_target::DrawTargetService;
use bones_kernel::logging::Logger;
use sdl3::video::Window;
use send_wrapper::SendWrapper;

use crate::inner::Inner;
use crate::shared_inner::SharedInner;

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
    // `Arc` internally, `Engine::build`'s own reasoning for handing it to
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
        }))
    }

    pub fn present(&mut self) {
        self.0.inner_mut().canvas.present();
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
