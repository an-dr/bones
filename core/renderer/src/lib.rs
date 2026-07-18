//! Renderer module (design/modules.md, ADR-002, ADR-017): executes `gfx/*`
//! draw commands against one SDL window. A `bus::Module`: construction is
//! two-stage — `new` takes only a `Logger`, real SDL setup waits for
//! `init` to consume the `window-surface` service (the window itself,
//! provided by whoever configured one — `Engine::build` today).

mod ui_mesh;

use std::collections::HashMap;

use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont, point};
use bones_messages::gfx::{Command, DrawCircle, DrawLine, DrawRect, DrawSprite};
use bus::{Envelope, Handler, Module, ModuleContext};
use logging::Logger;
use sdl3::image::LoadTexture;
use sdl3::pixels::{Color, FColor, PixelFormat};
use sdl3::rect::Rect;
use sdl3::render::{Canvas, FPoint, Texture, TextureCreator, Vertex};
use sdl3::video::{Window, WindowContext};
use send_wrapper::SendWrapper;

pub use ui_mesh::{UiMesh, UiVertex};

/// A `gfx/*` draw command that goes through the retained-batch/layer
/// pipeline (as opposed to `Clear`/`LoadSprite`/`SetCamera`, which apply
/// immediately as retained singletons — see `Inner::record`).
// No `Copy`: `Text` owns a `String` (the source `DrawText<'a>`'s `&str`
// borrows the envelope payload, which doesn't outlive one `record` call).
#[derive(Clone)]
enum RetainedDraw {
    Sprite(DrawSprite),
    Rect(DrawRect),
    Line(DrawLine),
    Circle(DrawCircle),
    Text {
        text: String,
        x: i32,
        y: i32,
        size: u16,
        color: (u8, u8, u8, u8),
        layer: u8,
    },
}

impl RetainedDraw {
    fn layer(&self) -> u8 {
        match self {
            Self::Sprite(d) => d.layer,
            Self::Rect(d) => d.layer,
            Self::Line(d) => d.layer,
            Self::Circle(d) => d.layer,
            Self::Text { layer, .. } => *layer,
        }
    }
}

// `radius` comes from an untrusted extension's `DrawCircle` payload. Both
// circle helpers below loop and do `2 * radius` arithmetic proportional to
// it; without a cap, a hostile or buggy extension could overflow that
// arithmetic or hang the render phase, which — unlike an extension — has no
// time budget (design/modules.md's trust model) and would stall the engine.
// Far larger than any real screen, so this never clips an intentional draw.
const MAX_CIRCLE_RADIUS: i32 = 10_000;

/// Points approximating a circle outline, centered on `(cx, cy)` with the
/// given `radius`, via the midpoint circle algorithm's 8-way symmetry.
fn circle_outline_points(cx: i32, cy: i32, radius: i32) -> Vec<FPoint> {
    let radius = radius.clamp(0, MAX_CIRCLE_RADIUS);
    let mut points = Vec::new();
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    while x >= y {
        for (dx, dy) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            points.push(FPoint::new((cx + dx) as f32, (cy + dy) as f32));
        }
        y += 1;
        err += 1 + 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
    points
}

/// Horizontal scanlines (as endpoint pairs) filling a circle centered on
/// `(cx, cy)` with the given `radius`, one line per row.
fn circle_fill_lines(cx: i32, cy: i32, radius: i32) -> Vec<(FPoint, FPoint)> {
    let radius = radius.clamp(0, MAX_CIRCLE_RADIUS);
    let mut lines = Vec::with_capacity((2 * radius + 1) as usize);
    for dy in -radius..=radius {
        let half_width = ((radius * radius - dy * dy).max(0) as f32).sqrt().round() as i32;
        lines.push((
            FPoint::new((cx - half_width) as f32, (cy + dy) as f32),
            FPoint::new((cx + half_width) as f32, (cy + dy) as f32),
        ));
    }
    lines
}

// `text`/`size` come from an untrusted extension's `DrawText` payload.
// Unclamped, a hostile or buggy extension could request a rasterization
// buffer proportional to chars * size^2 — large enough to hang or OOM the
// render phase, which (unlike an extension) has no time budget
// (design/modules.md's trust model). Both caps are far larger than any
// real one-line label, so neither clips an intentional draw.
const MAX_TEXT_SIZE_PX: f32 = 256.0;
const MAX_TEXT_CHARS: usize = 512;

/// Rasterizes one line of `text` at `size` px into a straight-alpha RGBA8
/// buffer sized to its own bounding box, tinted `color`. No wrapping, no
/// glyph/atlas cache — text draws are rare enough per frame that
/// re-rasterizing each one is simpler and not worth the cache-invalidation
/// cost (same reasoning as the sprite/shape draws' lack of a cache).
fn rasterize_text(font: &FontRef, text: &str, size: f32, color: (u8, u8, u8, u8)) -> (u32, u32, Vec<u8>) {
    let size = size.clamp(1.0, MAX_TEXT_SIZE_PX);
    let scaled = font.as_scaled(PxScale::from(size));
    let ascent = scaled.ascent();
    let height = scaled.height().ceil().max(1.0) as u32;

    let mut glyphs = Vec::new();
    let mut cursor = 0.0f32;
    for c in text.chars().take(MAX_TEXT_CHARS) {
        let id = scaled.glyph_id(c);
        glyphs.push(id.with_scale_and_position(PxScale::from(size), point(cursor, ascent)));
        cursor += scaled.h_advance(id);
    }
    let width = cursor.ceil().max(1.0) as u32;

    let (r, g, b, a) = color;
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    for glyph in glyphs {
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px as u32 >= width || py as u32 >= height {
                    return;
                }
                let idx = ((py as u32 * width + px as u32) * 4) as usize;
                buffer[idx] = r;
                buffer[idx + 1] = g;
                buffer[idx + 2] = b;
                buffer[idx + 3] = (a as f32 * coverage).round() as u8;
            });
        }
    }
    (width, height, buffer)
}

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
    /// Handles one decoded `gfx/*` command from `sender`. Cache-populating
    /// commands (`Clear`, `LoadSprite`) apply immediately since they're
    /// idempotent state, not a per-frame draw; `DrawSprite` only
    /// accumulates into `pending_draws` — actual drawing happens in
    /// `composite`, once per frame.
    fn record(&mut self, sender: &str, command: Command) -> Result<(), String> {
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
                self.pending_draws.entry(sender.to_string()).or_default().push(RetainedDraw::Text {
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
    fn composite(&mut self) -> Result<(), String> {
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
            let draws = self.retained_draws.get(sender).expect("sender_order and retained_draws stay in sync");
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
                        .copy_ex(texture, src, dst, draw.angle as f64, None::<FPoint>, draw.flip_h, draw.flip_v)
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
                        .draw_line(FPoint::new(x1 as f32, y1 as f32), FPoint::new(x2 as f32, y2 as f32))
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
                        self.canvas.draw_points(points.as_slice()).map_err(|e| e.to_string())?;
                    }
                }
                RetainedDraw::Text { text, x, y, size, color, .. } => {
                    if text.is_empty() {
                        continue;
                    }
                    let (width, height, rgba) = rasterize_text(&self.font, text, *size as f32, *color);
                    let mut texture = self
                        .texture_creator
                        .create_texture_streaming(PixelFormat::RGBA32, width, height)
                        .map_err(|e| e.to_string())?;
                    texture.set_blend_mode(sdl3::render::BlendMode::Blend);
                    texture.update(None, &rgba, width as usize * 4).map_err(|e| e.to_string())?;

                    let (screen_x, screen_y) = to_screen(*x, *y);
                    let screen_w = (width as f32 * zoom).round().max(0.0) as u32;
                    let screen_h = (height as f32 * zoom).round().max(0.0) as u32;
                    self.canvas
                        .copy(&texture, None, Rect::new(screen_x, screen_y, screen_w, screen_h))
                        .map_err(|e| e.to_string())?;
                }
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
/// (not silent UB) if `State` is ever actually touched from a thread other
/// than the one that created it — true today, since dispatch is single-
/// threaded, but enforced rather than merely assumed.
pub struct Renderer(SendWrapper<State>);

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

        self.0.inner = Some(Inner {
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
        });
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
