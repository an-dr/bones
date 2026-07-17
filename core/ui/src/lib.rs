//! egui UI module (ADR-005, design/presentation.md): decodes `ui/spec`
//! messages into an embedded `egui::Context`, publishes `ui/clicked` and
//! `ui/changed` back, and submits the tessellated output to the renderer
//! directly — design/modules.md's `draw-target` service, direct-wired for
//! now the same way `renderer` itself is wired into `Engine` rather than
//! through a generic module trait (see docs/structure.md).
//!
//! TODO: `ui/clicked`/`ui/changed` are broadcast on shared topics, not
//! targeted to the owning extension only (presentation.md's stated
//! contract) — every extension subscribed to `ui/*` sees every event and
//! must filter by its own widget ids. Fine while one extension uses `ui/*`
//! at a time; revisit (direct send, or a per-extension topic) once that
//! stops holding.

use std::collections::HashMap;
use std::time::Instant;

use bones_messages::ui::{Changed, Clicked, Spec, Widget};
use bones_messages::{DecodeMessage, EncodeMessage};
use bus::{Bus, Envelope, Handler};
use logging::Logger;
use renderer::{Renderer, UiMesh, UiVertex};

enum OwnedWidget {
    Label(String),
    TextEdit { id: u32, text: String },
    Button { id: u32, label: String },
}

struct PendingSpec {
    title: String,
    widgets: Vec<OwnedWidget>,
}

impl PendingSpec {
    fn from_message(spec: &Spec<'_>) -> Self {
        Self {
            title: spec.title.to_string(),
            widgets: spec
                .widgets
                .iter()
                .map(|widget| match widget {
                    Widget::Label { text } => OwnedWidget::Label((*text).to_string()),
                    Widget::TextEdit { id, text } => OwnedWidget::TextEdit {
                        id: *id,
                        text: (*text).to_string(),
                    },
                    Widget::Button { id, label } => OwnedWidget::Button {
                        id: *id,
                        label: (*label).to_string(),
                    },
                })
                .collect(),
        }
    }
}

/// Owns the embedded egui context: decodes widget specs published on
/// `ui/spec`, runs one egui frame per tick, and drives the renderer's
/// ui-mesh submission.
pub struct Ui {
    ctx: egui::Context,
    bus: Bus,
    // Keyed by the publishing extension's bus sender name; ADR-005 is
    // immediate-mode, so a spec not republished this frame draws nothing —
    // `update` drains this on every call.
    pending: HashMap<String, PendingSpec>,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    pointer_pos: egui::Pos2,
    start: Instant,
    logger: Logger,
}

impl Ui {
    /// Starts with an empty egui context and no pending specs; `bus` is
    /// used both to publish `ui/clicked`/`ui/changed` and to register `Self`
    /// as a bus endpoint via `Handler` (the caller's job, not `new`'s).
    pub fn new(bus: Bus, logger: Logger) -> Self {
        Self {
            ctx: egui::Context::default(),
            bus,
            pending: HashMap::new(),
            events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            pointer_pos: egui::Pos2::ZERO,
            start: Instant::now(),
            logger,
        }
    }

    /// Whether egui currently wants pointer events — reflects hover/focus
    /// state as of the end of the last `update` call, one frame stale by
    /// construction (egui itself only knows what it wants after laying out
    /// a frame). The standard technique custom egui backends use to decide
    /// per-event ADR-008 consumption without waiting for the next frame.
    pub fn wants_pointer_input(&self) -> bool {
        self.ctx.wants_pointer_input()
    }

    /// Same staleness caveat as `wants_pointer_input`, for the keyboard.
    pub fn wants_keyboard_input(&self) -> bool {
        self.ctx.wants_keyboard_input()
    }

    /// Translates one raw SDL event into egui input, buffering it for the
    /// next `update`. Returns whether this layer claims the event (ADR-008)
    /// — the caller (platform's pre-consumption hook) uses this to decide
    /// whether the event still reaches `input/*`.
    pub fn feed_event(&mut self, event: &sdl3::event::Event) -> bool {
        use sdl3::event::Event as E;
        match event {
            E::MouseMotion { x, y, .. } => {
                self.pointer_pos = egui::pos2(*x, *y);
                self.events.push(egui::Event::PointerMoved(self.pointer_pos));
                self.wants_pointer_input()
            }
            E::MouseButtonDown { mouse_btn, x, y, .. } | E::MouseButtonUp { mouse_btn, x, y, .. } => {
                let Some(button) = translate_mouse_button(*mouse_btn) else {
                    return false;
                };
                self.pointer_pos = egui::pos2(*x, *y);
                self.events.push(egui::Event::PointerButton {
                    pos: self.pointer_pos,
                    button,
                    pressed: matches!(event, E::MouseButtonDown { .. }),
                    modifiers: self.modifiers,
                });
                self.wants_pointer_input()
            }
            E::TextInput { text, .. } => {
                self.events.push(egui::Event::Text(text.clone()));
                true
            }
            E::KeyDown { keycode: Some(key), repeat, keymod, .. }
            | E::KeyUp { keycode: Some(key), repeat, keymod, .. } => {
                let Some(mapped) = translate_key(*key) else {
                    return false;
                };
                self.modifiers = compute_modifiers(*keymod);
                self.events.push(egui::Event::Key {
                    key: mapped,
                    physical_key: None,
                    pressed: matches!(event, E::KeyDown { .. }),
                    repeat: *repeat,
                    modifiers: self.modifiers,
                });
                self.wants_keyboard_input()
            }
            _ => false,
        }
    }

    /// Runs one egui frame and draws it via `renderer`. Caller must call
    /// this after `gfx/*` draws for the frame (ui draws above all gfx
    /// layers, design/presentation.md) and before `renderer.present()`.
    pub fn update(&mut self, renderer: &mut Renderer, screen_width: u32, screen_height: u32) {
        let screen_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(screen_width as f32, screen_height as f32),
        );
        // TODO: no `native_pixels_per_point`/`pixels_per_point` set — egui
        // defaults to 1.0, so a scaled (high-DPI) display renders undersized
        // relative to the physical window.
        let raw_input = egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(self.start.elapsed().as_secs_f64()),
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };

        let pending = std::mem::take(&mut self.pending);
        let bus = &self.bus;
        let full_output = self.ctx.run(raw_input, |ctx| {
            for (sender, spec) in &pending {
                egui::Window::new(spec.title.as_str())
                    .id(egui::Id::new(sender.as_str()))
                    .show(ctx, |ui| {
                        for widget in &spec.widgets {
                            match widget {
                                OwnedWidget::Label(text) => {
                                    ui.label(text);
                                }
                                OwnedWidget::TextEdit { id, text } => {
                                    let mut buf = text.clone();
                                    if ui.text_edit_singleline(&mut buf).changed() {
                                        publish(bus, Changed { id: *id, text: &buf });
                                    }
                                }
                                OwnedWidget::Button { id, label } => {
                                    if ui.button(label).clicked() {
                                        publish(bus, Clicked { id: *id });
                                    }
                                }
                            }
                        }
                    });
            }
        });

        for (id, delta) in &full_output.textures_delta.set {
            let egui::ImageData::Color(image) = &delta.image;
            let rgba = convert_color_image_to_straight_rgba(image);
            let key = compute_texture_key(*id);
            let width = image.size[0] as u32;
            let height = image.size[1] as u32;
            let result = match delta.pos {
                None => renderer.set_ui_texture(key, width, height, &rgba),
                Some([x, y]) => renderer.update_ui_texture_region(key, x as u32, y as u32, width, height, &rgba),
            };
            if let Err(err) = result {
                self.logger.error("ui", &format!("uploading texture {id:?}: {err}"));
            }
        }

        let clipped = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for primitive in &clipped {
            let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                continue; // TODO: PaintCallback (custom GPU code) primitives aren't supported.
            };
            if mesh.indices.is_empty() {
                continue;
            }
            let clip = primitive.clip_rect;
            let ui_mesh = UiMesh {
                vertices: mesh
                    .vertices
                    .iter()
                    .map(|vertex| {
                        let [r, g, b, a] = vertex.color.to_srgba_unmultiplied();
                        UiVertex {
                            x: vertex.pos.x,
                            y: vertex.pos.y,
                            u: vertex.uv.x,
                            v: vertex.uv.y,
                            r,
                            g,
                            b,
                            a,
                        }
                    })
                    .collect(),
                indices: mesh.indices.clone(),
                texture: compute_texture_key(mesh.texture_id),
                clip: (
                    clip.min.x.round() as i32,
                    clip.min.y.round() as i32,
                    clip.width().round().max(0.0) as u32,
                    clip.height().round().max(0.0) as u32,
                ),
            };
            if let Err(err) = renderer.draw_ui_mesh(&ui_mesh) {
                self.logger.error("ui", &format!("drawing ui mesh: {err}"));
            }
        }

        for id in &full_output.textures_delta.free {
            renderer.free_ui_texture(compute_texture_key(*id));
        }
    }
}

impl Handler for Ui {
    fn handle(&mut self, envelope: &Envelope) {
        match Spec::decode(&envelope.payload) {
            Ok(spec) => {
                self.pending.insert(envelope.sender.clone(), PendingSpec::from_message(&spec));
            }
            Err(err) => {
                self.logger
                    .error("ui", &format!("{} from '{}': {err}", envelope.topic, envelope.sender));
            }
        }
    }
}

fn publish<M: EncodeMessage>(bus: &Bus, message: M) {
    bus.publish(Envelope {
        topic: M::TOPIC.to_string(),
        sender: "ui".to_string(),
        correlation: None,
        payload: message.encode(),
    });
}

/// Splits egui's own texture-id namespace into one `u64` key: `Managed` and
/// `User` ids each start at 0, so they'd otherwise collide.
fn compute_texture_key(id: egui::TextureId) -> u64 {
    match id {
        egui::TextureId::Managed(n) => n << 1,
        egui::TextureId::User(n) => (n << 1) | 1,
    }
}

/// `Color32` is always premultiplied-alpha (epaint's own convention, both
/// for mesh vertex colors and image pixel data); converted to straight
/// alpha here so the renderer can use SDL's standard (non-premultiplied)
/// alpha-blend mode consistently for every ui draw call.
fn convert_color_image_to_straight_rgba(image: &egui::ColorImage) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        bytes.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }
    bytes
}

fn translate_mouse_button(button: sdl3::mouse::MouseButton) -> Option<egui::PointerButton> {
    match button {
        sdl3::mouse::MouseButton::Left => Some(egui::PointerButton::Primary),
        sdl3::mouse::MouseButton::Right => Some(egui::PointerButton::Secondary),
        sdl3::mouse::MouseButton::Middle => Some(egui::PointerButton::Middle),
        _ => None,
    }
}

/// Only the handful of keys a single-line text field needs to navigate and
/// edit; egui receives typed characters separately via `Event::Text`.
fn translate_key(keycode: sdl3::keyboard::Keycode) -> Option<egui::Key> {
    use sdl3::keyboard::Keycode as K;
    match keycode {
        K::Backspace => Some(egui::Key::Backspace),
        K::Delete => Some(egui::Key::Delete),
        K::Return | K::Return2 | K::KpEnter => Some(egui::Key::Enter),
        K::Escape => Some(egui::Key::Escape),
        K::Tab => Some(egui::Key::Tab),
        K::Left => Some(egui::Key::ArrowLeft),
        K::Right => Some(egui::Key::ArrowRight),
        K::Up => Some(egui::Key::ArrowUp),
        K::Down => Some(egui::Key::ArrowDown),
        K::Home => Some(egui::Key::Home),
        K::End => Some(egui::Key::End),
        _ => None,
    }
}

fn compute_modifiers(keymod: sdl3::keyboard::Mod) -> egui::Modifiers {
    use sdl3::keyboard::Mod;
    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    egui::Modifiers {
        alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
        ctrl,
        shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        mac_cmd: false,
        command: ctrl,
    }
}
