//! Renderer module (design/modules.md, ADR-002): executes `gfx/*` draw
//! commands against one SDL window. TODO: not a generic Module yet —
//! directly wired into Engine, same as host and platform, until the module
//! trait exists to design from real examples.

mod ui_mesh;

use std::collections::HashMap;

use bones_messages::gfx::Command;
use bus::{Envelope, Handler};
use logging::Logger;
use sdl3::image::LoadTexture;
use sdl3::pixels::{Color, FColor, PixelFormat};
use sdl3::rect::Rect;
use sdl3::render::{Canvas, FPoint, Texture, TextureCreator, Vertex};
use sdl3::video::{Window, WindowContext};
use send_wrapper::SendWrapper;

pub use ui_mesh::{UiMesh, UiVertex};

struct Inner {
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    // `unsafe_textures` (sdl3 feature): a `Texture<'r>` borrowing from
    // `texture_creator` can't be cached in the same struct as its own
    // creator without it — the crate's sanctioned way to keep textures
    // across frames instead of a self-referential struct.
    textures: HashMap<u32, Texture>,
    // Separate id space from `textures` (gfx sprite ids are extension-
    // assigned `u32`s; ui texture ids are egui's own `u64` texture ids).
    ui_textures: HashMap<u64, Texture>,
    logger: Logger,
}

impl Inner {
    fn execute(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::Clear(clear) => {
                let (r, g, b, a) = (clear.r, clear.g, clear.b, clear.a);
                self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                self.canvas.clear();
            }
            Command::LoadSprite(load) => {
                let texture = self
                    .texture_creator
                    .load_texture_bytes(load.png_bytes)
                    .map_err(|e| e.to_string())?;
                self.textures.insert(load.id, texture);
            }
            Command::DrawSprite(draw) => {
                let texture = self
                    .textures
                    .get(&draw.id)
                    .ok_or_else(|| format!("unknown sprite id {}", draw.id))?;
                let src = Rect::new(draw.src_x, draw.src_y, draw.src_w, draw.src_h);
                let dst = Rect::new(draw.dst_x, draw.dst_y, draw.src_w, draw.src_h);
                self.canvas
                    .copy(texture, src, dst)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn set_ui_texture(&mut self, id: u64, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        let mut texture = self
            .texture_creator
            .create_texture_streaming(PixelFormat::RGBA32, width, height)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(sdl3::render::BlendMode::Blend);
        texture
            .update(None, rgba, width as usize * 4)
            .map_err(|e| e.to_string())?;
        self.ui_textures.insert(id, texture);
        Ok(())
    }

    fn free_ui_texture(&mut self, id: u64) {
        self.ui_textures.remove(&id);
    }

    fn update_ui_texture_region(
        &mut self,
        id: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        let texture = self
            .ui_textures
            .get_mut(&id)
            .ok_or_else(|| format!("unknown ui texture id {id}"))?;
        texture
            .update(Rect::new(x as i32, y as i32, width, height), rgba, width as usize * 4)
            .map_err(|e| e.to_string())
    }

    fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
        let texture = self
            .ui_textures
            .get(&mesh.texture)
            .ok_or_else(|| format!("unknown ui texture id {}", mesh.texture))?;
        let vertices: Vec<Vertex> = mesh
            .vertices
            .iter()
            .map(|v| Vertex {
                position: FPoint::new(v.x, v.y),
                color: FColor {
                    r: v.r as f32 / 255.0,
                    g: v.g as f32 / 255.0,
                    b: v.b as f32 / 255.0,
                    a: v.a as f32 / 255.0,
                },
                tex_coord: FPoint::new(v.u, v.v),
            })
            .collect();
        let (cx, cy, cw, ch) = mesh.clip;
        self.canvas.set_clip_rect(Some(Rect::new(cx, cy, cw, ch)));
        let result = self
            .canvas
            .render_geometry(&vertices, Some(texture), &mesh.indices)
            .map_err(|e| e.to_string());
        self.canvas.set_clip_rect(None);
        result
    }
}

/// SDL's `Window`/`Canvas` aren't `Send`/`Sync` (real thread-affinity
/// constraints on some platforms), but the vendored `pubsub-bus` crate
/// requires both on anything registered as a bus endpoint. `SendWrapper`
/// makes the type check pass while keeping the guarantee real: it panics
/// (not silent UB) if `Inner` is ever actually touched from a thread other
/// than the one that created it — true today, since dispatch is single-
/// threaded, but enforced rather than merely assumed.
pub struct Renderer(SendWrapper<Inner>);

impl Renderer {
    pub fn new(window: Window, logger: Logger) -> Self {
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        Self(SendWrapper::new(Inner {
            canvas,
            texture_creator,
            textures: HashMap::new(),
            ui_textures: HashMap::new(),
            logger,
        }))
    }

    pub fn present(&mut self) {
        self.0.canvas.present();
    }

    /// Current window size in pixels, for callers (the ui module) that need
    /// to size their own output to match without holding a window handle.
    pub fn size(&self) -> (u32, u32) {
        self.0.canvas.window().size()
    }

    /// Registers or fully replaces the RGBA8 (straight alpha) texture the
    /// ui module addresses as `id` in `UiMesh::texture` — egui's own
    /// texture ids (font atlas plus any user textures). `rgba.len()` must
    /// be `width * height * 4`.
    pub fn set_ui_texture(&mut self, id: u64, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        self.0.set_ui_texture(id, width, height, rgba)
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
        self.0.update_ui_texture_region(id, x, y, width, height, rgba)
    }

    /// Drops a texture registered by `set_ui_texture` (egui's `TexturesDelta::free`).
    pub fn free_ui_texture(&mut self, id: u64) {
        self.0.free_ui_texture(id);
    }

    /// Draws one textured triangle mesh from egui's tessellated output
    /// (design/presentation.md: "egui output is drawn by the renderer
    /// above all gfx layers") — the caller is responsible for calling this
    /// after every `gfx/*` draw for the frame has already executed.
    pub fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
        self.0.draw_ui_mesh(mesh)
    }
}

impl Handler for Renderer {
    fn handle(&mut self, envelope: &Envelope) {
        // Only ever reached for gfx/* (the subscription), so an unmatched
        // topic here is always a caller mistake (e.g. a typo'd command
        // name) worth surfacing rather than silently dropping.
        let result = match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => self.0.execute(command),
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
