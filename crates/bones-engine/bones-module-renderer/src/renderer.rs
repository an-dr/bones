use ab_glyph::FontRef;
use bones_messages::gfx::Command;
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::renderer::{DisplayChanged, LogicalCanvas};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel::logging::Logger;
use sdl3::video::Window;
use send_wrapper::SendWrapper;

use crate::inner::Inner;
use crate::ui_mesh::UiMesh;

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
    inner: Option<Inner>,
}

impl State {
    fn inner_mut(&mut self) -> &mut Inner {
        self.inner
            .as_mut()
            .expect("Renderer used before Module::init built its SDL state")
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

    /// Current window size in pixels, for callers (the ui module) that need
    /// to size their own output to match without holding a window handle.
    pub fn size(&self) -> (u32, u32) {
        self.0
            .inner
            .as_ref()
            .expect("Renderer used before Module::init built its SDL state")
            .canvas
            .window()
            .size()
    }

    /// Registers or fully replaces the RGBA8 (straight alpha) texture the
    /// ui module addresses as `id` in `UiMesh::texture` — egui's own
    /// texture ids (font atlas plus any user textures). `rgba.len()` must
    /// be `width * height * 4`.
    pub fn set_ui_texture(
        &mut self,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.0.inner_mut().set_ui_texture(id, width, height, rgba)
    }

    /// Patches a sub-rectangle of a texture already registered by
    /// `set_ui_texture` (egui's `ImageDelta::pos`, e.g. the font atlas
    /// growing as new glyphs are rasterized). Errors if `id` was never
    /// `set_ui_texture`-created first.
    pub fn update_ui_texture_region(
        &mut self,
        id: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.0
            .inner_mut()
            .update_ui_texture_region(id, x, y, width, height, rgba)
    }

    /// Drops a texture registered by `set_ui_texture` (egui's `TexturesDelta::free`).
    pub fn free_ui_texture(&mut self, id: u64) {
        self.0.inner_mut().free_ui_texture(id);
    }

    /// Draws one textured triangle mesh from egui's tessellated output
    /// (design/presentation.md: "egui output is drawn by the renderer
    /// above all gfx layers") — the caller is responsible for calling this
    /// after every `gfx/*` draw for the frame has already executed.
    pub fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
        self.0.inner_mut().draw_ui_mesh(mesh)
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

    /// Consumes the `window-surface` service (design/modules.md) and
    /// builds the real SDL canvas/texture-creator state; errors if no
    /// window was provided (e.g. `.renderer()` without `.window(...)`).
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        let window = ctx.consume_service::<Window>().ok_or_else(|| {
            "renderer needs a window-surface service (configure .window(...))".to_string()
        })?;
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();

        let font = FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR)
            .map_err(|e| e.to_string())?;

        self.0.inner = Some(Inner::new(canvas, texture_creator, font));
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
