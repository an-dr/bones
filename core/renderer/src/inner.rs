use std::collections::HashMap;

use ab_glyph::FontRef;
use bones_messages::gfx::Command;
use sdl3::image::LoadTexture;
use sdl3::pixels::{Color, FColor, PixelFormat};
use sdl3::rect::Rect;
use sdl3::render::{Canvas, FPoint, Texture, TextureCreator, Vertex};
use sdl3::video::{Window, WindowContext};

use crate::circle_geometry::{circle_fill_lines, circle_outline_points};
use crate::retained_draw::RetainedDraw;
use crate::text_rasterizer::rasterize_text;
use crate::ui_mesh::UiMesh;

pub(crate) struct Inner {
    pub(crate) canvas: Canvas<Window>,
    pub(crate) texture_creator: TextureCreator<WindowContext>,
    // `unsafe_textures` (sdl3 feature): a `Texture<'r>` borrowing from
    // `texture_creator` can't be cached in the same struct as its own
    // creator without it — the crate's sanctioned way to keep textures
    // across frames instead of a self-referential struct.
    textures: HashMap<u32, Texture>,
    // Separate id space from `textures` (gfx sprite ids are extension-
    // assigned `u32`s; ui texture ids are egui's own `u64` texture ids).
    ui_textures: HashMap<u64, Texture>,
    // Retained gfx/* state (design/presentation.md): the most recent clear
    // color, and each sender's most recently *completed* draw batch —
    // composited every frame regardless of whether that sender published
    // again, so a paused extension keeps its last frame visible. `pending`
    // accumulates the batch currently being received; `composite` swaps it
    // into `retained` (full replacement, not a merge) once per frame.
    clear_color: Option<Color>,
    pending_draws: HashMap<String, Vec<RetainedDraw>>,
    retained_draws: HashMap<String, Vec<RetainedDraw>>,
    // `HashMap` iteration order is unspecified, so composite order across
    // senders can't come from `retained_draws` directly (ADR-009 promises
    // per-sender FIFO, not cross-sender nondeterminism). Records each
    // sender's first-seen order; composite draws in this order.
    sender_order: Vec<String>,
    // World-to-screen camera: (x, y, zoom), `gfx/set-camera`. Global (one
    // viewport for the scene), retained like `clear_color` — last writer
    // wins. Defaults to an identity transform.
    camera: (f32, f32, f32),
    // Pure-Rust rasterization (ab_glyph) rather than SDL3's `ttf` feature:
    // that feature links `sdl3-ttf-sys`, which bundles HarfBuzz, which
    // fails to link in this workspace's dev profile on this toolchain
    // (MSVC ARM64: unresolved `_CrtDbgReport` from harfbuzz.cc.obj — a
    // debug/release CRT mismatch between cargo's dev profile and the
    // vendored C++ build; `--release` links fine, dev does not, which
    // breaks the default `cargo build`/`cargo test` workflow). ab_glyph is
    // already resolved transitively via egui, so this adds no new
    // dependency risk. One embedded font (Hack-Regular, already vendored
    // via egui — see Cargo.toml).
    font: FontRef<'static>,
}

impl Inner {
    pub(crate) fn new(
        canvas: Canvas<Window>,
        texture_creator: TextureCreator<WindowContext>,
        font: FontRef<'static>,
    ) -> Self {
        Self {
            canvas,
            texture_creator,
            textures: HashMap::new(),
            ui_textures: HashMap::new(),
            clear_color: None,
            pending_draws: HashMap::new(),
            retained_draws: HashMap::new(),
            sender_order: Vec::new(),
            camera: (0.0, 0.0, 1.0),
            font,
        }
    }

    /// Handles one decoded `gfx/*` command from `sender`. Cache-populating
    /// commands (`Clear`, `LoadSprite`) apply immediately since they're
    /// idempotent state, not a per-frame draw; `DrawSprite` only
    /// accumulates into `pending_draws` — actual drawing happens in
    /// `composite`, once per frame.
    pub(crate) fn record(&mut self, sender: &str, command: Command) -> Result<(), String> {
        match command {
            Command::Clear(clear) => {
                let (r, g, b, a) = (clear.r, clear.g, clear.b, clear.a);
                self.clear_color = Some(Color::RGBA(r, g, b, a));
            }
            Command::LoadSprite(load) => {
                let texture = self
                    .texture_creator
                    .load_texture_bytes(load.png_bytes)
                    .map_err(|e| e.to_string())?;
                self.textures.insert(load.id, texture);
            }
            Command::DrawSprite(draw) => {
                self.pending_draws
                    .entry(sender.to_string())
                    .or_default()
                    .push(RetainedDraw::Sprite(draw));
            }
            Command::SetCamera(camera) => {
                self.camera = (camera.x, camera.y, camera.zoom);
            }
            Command::DrawRect(draw) => {
                self.pending_draws
                    .entry(sender.to_string())
                    .or_default()
                    .push(RetainedDraw::Rect(draw));
            }
            Command::DrawLine(draw) => {
                self.pending_draws
                    .entry(sender.to_string())
                    .or_default()
                    .push(RetainedDraw::Line(draw));
            }
            Command::DrawCircle(draw) => {
                self.pending_draws
                    .entry(sender.to_string())
                    .or_default()
                    .push(RetainedDraw::Circle(draw));
            }
            Command::DrawText(draw) => {
                self.pending_draws
                    .entry(sender.to_string())
                    .or_default()
                    .push(RetainedDraw::Text {
                        text: draw.text.to_string(),
                        x: draw.x,
                        y: draw.y,
                        size: draw.size,
                        color: draw.color,
                        layer: draw.layer,
                    });
            }
        }
        Ok(())
    }

    /// Runs once per frame (the renderer's `render` phase, before ui draws
    /// above it and before `present`): swaps this frame's completed
    /// per-sender batches into `retained_draws`, then draws the clear
    /// color followed by every retained draw — ordered by `layer`
    /// (ties broken by sender-arrival order, since `sort_by_key` is
    /// stable) — through the current camera transform.
    pub(crate) fn composite(&mut self) -> Result<(), String> {
        if let Some(color) = self.clear_color {
            self.canvas.set_draw_color(color);
            self.canvas.clear();
        }
        for (sender, draws) in self.pending_draws.drain() {
            if !self.retained_draws.contains_key(&sender) {
                self.sender_order.push(sender.clone());
            }
            self.retained_draws.insert(sender, draws);
        }

        let mut ordered: Vec<RetainedDraw> = Vec::new();
        for sender in &self.sender_order {
            let draws = self
                .retained_draws
                .get(sender)
                .expect("sender_order and retained_draws stay in sync");
            ordered.extend_from_slice(draws);
        }
        ordered.sort_by_key(|draw| draw.layer());

        let (camera_x, camera_y, zoom) = self.camera;
        let to_screen = |x: i32, y: i32| -> (i32, i32) {
            (
                ((x as f32 - camera_x) * zoom).round() as i32,
                ((y as f32 - camera_y) * zoom).round() as i32,
            )
        };
        for draw in &ordered {
            match draw {
                RetainedDraw::Sprite(draw) => {
                    let texture = self
                        .textures
                        .get_mut(&draw.id)
                        .ok_or_else(|| format!("unknown sprite id {}", draw.id))?;
                    let (tint_r, tint_g, tint_b, tint_a) = draw.tint;
                    texture.set_color_mod(tint_r, tint_g, tint_b);
                    texture.set_alpha_mod(tint_a);

                    let src = Rect::new(draw.src_x, draw.src_y, draw.src_w, draw.src_h);
                    let (screen_x, screen_y) = to_screen(draw.dst_x, draw.dst_y);
                    let screen_w = (draw.dst_w as f32 * zoom).round().max(0.0) as u32;
                    let screen_h = (draw.dst_h as f32 * zoom).round().max(0.0) as u32;
                    let dst = Rect::new(screen_x, screen_y, screen_w, screen_h);
                    self.canvas
                        .copy_ex(
                            texture,
                            src,
                            dst,
                            draw.angle as f64,
                            None::<FPoint>,
                            draw.flip_h,
                            draw.flip_v,
                        )
                        .map_err(|e| e.to_string())?;
                }
                RetainedDraw::Rect(draw) => {
                    let (r, g, b, a) = draw.color;
                    self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                    let (screen_x, screen_y) = to_screen(draw.x, draw.y);
                    let screen_w = (draw.w as f32 * zoom).round().max(0.0) as u32;
                    let screen_h = (draw.h as f32 * zoom).round().max(0.0) as u32;
                    let rect = Rect::new(screen_x, screen_y, screen_w, screen_h);
                    let result = if draw.filled {
                        self.canvas.fill_rect(rect)
                    } else {
                        self.canvas.draw_rect(rect)
                    };
                    result.map_err(|e| e.to_string())?;
                }
                RetainedDraw::Line(draw) => {
                    let (r, g, b, a) = draw.color;
                    self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                    let (x1, y1) = to_screen(draw.x1, draw.y1);
                    let (x2, y2) = to_screen(draw.x2, draw.y2);
                    self.canvas
                        .draw_line(
                            FPoint::new(x1 as f32, y1 as f32),
                            FPoint::new(x2 as f32, y2 as f32),
                        )
                        .map_err(|e| e.to_string())?;
                }
                RetainedDraw::Circle(draw) => {
                    let (r, g, b, a) = draw.color;
                    self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                    let (cx, cy) = to_screen(draw.x, draw.y);
                    let radius = (draw.radius as f32 * zoom).round().max(0.0) as i32;
                    if draw.filled {
                        for (p1, p2) in circle_fill_lines(cx, cy, radius) {
                            self.canvas.draw_line(p1, p2).map_err(|e| e.to_string())?;
                        }
                    } else {
                        let points = circle_outline_points(cx, cy, radius);
                        self.canvas
                            .draw_points(points.as_slice())
                            .map_err(|e| e.to_string())?;
                    }
                }
                RetainedDraw::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                    ..
                } => {
                    if text.is_empty() {
                        continue;
                    }
                    let (width, height, rgba) =
                        rasterize_text(&self.font, text, *size as f32, *color);
                    let mut texture = self
                        .texture_creator
                        .create_texture_streaming(PixelFormat::RGBA32, width, height)
                        .map_err(|e| e.to_string())?;
                    texture.set_blend_mode(sdl3::render::BlendMode::Blend);
                    texture
                        .update(None, &rgba, width as usize * 4)
                        .map_err(|e| e.to_string())?;

                    let (screen_x, screen_y) = to_screen(*x, *y);
                    let screen_w = (width as f32 * zoom).round().max(0.0) as u32;
                    let screen_h = (height as f32 * zoom).round().max(0.0) as u32;
                    self.canvas
                        .copy(
                            &texture,
                            None,
                            Rect::new(screen_x, screen_y, screen_w, screen_h),
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn set_ui_texture(
        &mut self,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
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

    pub(crate) fn free_ui_texture(&mut self, id: u64) {
        self.ui_textures.remove(&id);
    }

    pub(crate) fn update_ui_texture_region(
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
            .update(
                Rect::new(x as i32, y as i32, width, height),
                rgba,
                width as usize * 4,
            )
            .map_err(|e| e.to_string())
    }

    pub(crate) fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
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
