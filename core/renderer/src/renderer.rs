use ab_glyph::FontRef;
use bones_messages::gfx::Command;
use bus::{Envelope, Handler, Module, ModuleContext};
use logging::Logger;
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
    // `None` until `Module::init` consumes `window-surface` and builds the
    // real SDL state — every other method panics if called first (a
    // caller/wiring bug, not a runtime condition to recover from).
    inner: Option<Inner>,
}

impl State {
    fn inner_mut(&mut self) -> &mut Inner {
        self.inner.as_mut().expect("Renderer used before Module::init built its SDL state")
    }
}

impl Renderer {
    pub fn new(logger: Logger) -> Self {
        Self(SendWrapper::new(State { logger, inner: None }))
    }

    pub fn present(&mut self) {
        self.0.inner_mut().canvas.present();
    }

    /// Current window size in pixels, for callers (the ui module) that need
    /// to size their own output to match without holding a window handle.
    pub fn size(&self) -> (u32, u32) {
        self.0.inner.as_ref().expect("Renderer used before Module::init built its SDL state").canvas.window().size()
    }

    /// Registers or fully replaces the RGBA8 (straight alpha) texture the
    /// ui module addresses as `id` in `UiMesh::texture` — egui's own
    /// texture ids (font atlas plus any user textures). `rgba.len()` must
    /// be `width * height * 4`.
    pub fn set_ui_texture(&mut self, id: u64, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
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
        self.0.inner_mut().update_ui_texture_region(id, x, y, width, height, rgba)
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
        // Only ever reached for gfx/* (the subscription), so an unmatched
        // topic here is always a caller mistake (e.g. a typo'd command
        // name) worth surfacing rather than silently dropping.
        let result = match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => self.0.inner_mut().record(&envelope.sender, command),
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

impl Module for Renderer {
    fn name(&self) -> &str {
        "renderer"
    }

    /// Consumes the `window-surface` service (design/modules.md) and
    /// builds the real SDL canvas/texture-creator state; errors if no
    /// window was provided (e.g. `.renderer()` without `.window(...)`).
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        let window = ctx
            .consume_service::<Window>()
            .ok_or_else(|| "renderer needs a window-surface service (configure .window(...))".to_string())?;
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();

        let font = FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR).map_err(|e| e.to_string())?;

        self.0.inner = Some(Inner::new(canvas, texture_creator, font));
        Ok(())
    }

    /// Composites this frame's retained `gfx/*` batches (clear, then every
    /// sender's most recent draw batch) before `ui` draws above them and
    /// `present` runs (design/modules.md's `render` phase).
    fn render(&mut self) {
        if let Err(err) = self.0.inner_mut().composite() {
            self.0.logger.error("renderer", &format!("composite: {err}"));
        }
    }

    fn present(&mut self) {
        Renderer::present(self);
    }
}
