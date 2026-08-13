use std::collections::HashMap;
use std::time::Instant;

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel::draw_target::{DrawTargetService, UiMesh, UiVertex};
use bones_kernel::logging::Logger;
use bones_messages::ui::{Changed, Clicked, Spec};
use bones_messages::{DecodeMessage, Message};

use crate::input_translation::{compute_modifiers, translate_key, translate_mouse_button};
use crate::output_translation::{
    compute_texture_key, convert_color_image_to_straight_rgba, publish,
};
use crate::owned_widget::OwnedWidget;
use crate::pending_spec::PendingSpec;

/// Owns the embedded egui context: decodes widget specs published on
/// `ui/spec`, runs one egui frame per tick, and submits the resulting
/// meshes to whatever surface provided `draw-target`.
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
    // `None` until `Module::init` consumes `draw-target`. Nothing this
    // module produces can be drawn without one, so `init` fails rather
    // than leaving it unset — the surface is a hard requirement, not an
    // optional enhancement.
    target: Option<DrawTargetService>,
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
            target: None,
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
                self.events
                    .push(egui::Event::PointerMoved(self.pointer_pos));
                self.wants_pointer_input()
            }
            E::MouseButtonDown {
                mouse_btn, x, y, ..
            }
            | E::MouseButtonUp {
                mouse_btn, x, y, ..
            } => {
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
            E::KeyDown {
                keycode: Some(key),
                repeat,
                keymod,
                ..
            }
            | E::KeyUp {
                keycode: Some(key),
                repeat,
                keymod,
                ..
            } => {
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

    /// Runs one egui frame and draws it through the `draw-target` service.
    /// Caller must call this after `gfx/*` draws for the frame (ui draws
    /// above all gfx layers, design/presentation.md) and before the target
    /// presents.
    ///
    /// A no-op before `Module::init` has supplied a target.
    pub fn update(&mut self) {
        // Size read up front and the target re-borrowed further down, so
        // the egui pass in between can still use `self` for the widget
        // specs, the bus, and the logger.
        let Some((screen_width, screen_height)) = self.target.as_ref().map(|target| target.size())
        else {
            return;
        };
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
                                        publish(
                                            bus,
                                            Changed {
                                                id: *id,
                                                text: &buf,
                                            },
                                        );
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

        let clipped = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let logger = self.logger.clone();
        // Not a second chance to bail: the early return above already
        // established there is a target. Panicking beats returning here,
        // which would silently discard a whole frame of tessellated output.
        let target = self
            .target
            .as_mut()
            .expect("target was checked at the top of update");

        for (id, delta) in &full_output.textures_delta.set {
            let egui::ImageData::Color(image) = &delta.image;
            let rgba = convert_color_image_to_straight_rgba(image);
            let key = compute_texture_key(*id);
            let width = image.size[0] as u32;
            let height = image.size[1] as u32;
            let result = match delta.pos {
                None => target.set_ui_texture(key, width, height, &rgba),
                Some([x, y]) => {
                    target.update_ui_texture_region(key, x as u32, y as u32, width, height, &rgba)
                }
            };
            if let Err(err) = result {
                logger.error("ui", &format!("uploading texture {id:?}: {err}"));
            }
        }

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
            if let Err(err) = target.draw_ui_mesh(&ui_mesh) {
                logger.error("ui", &format!("drawing ui mesh: {err}"));
            }
        }

        for id in &full_output.textures_delta.free {
            target.free_ui_texture(compute_texture_key(*id));
        }
    }
}

impl Module for Ui {
    fn name(&self) -> &str {
        "ui"
    }

    /// Consumes the `draw-target` service — whatever module provides a
    /// surface, which is `renderer` in the default composition but need
    /// not be. Errors if none was provided, since this module has nothing
    /// to draw on without one.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        self.target = Some(ctx.consume_service::<DrawTargetService>().ok_or_else(|| {
            "ui needs a draw-target service (configure .renderer() too)".to_string()
        })?);
        ctx.subscribe(Spec::TOPIC);
        Ok(())
    }

    /// Claims the events egui is currently interested in (ADR-008), and
    /// buffers every event either way — the ones it does not claim still
    /// inform egui's next frame, they simply also reach `input/*`.
    fn filter_event(&mut self, event: &sdl3::event::Event) -> bool {
        self.feed_event(event)
    }

    /// Draws this frame's egui output above every `gfx/*` batch the target
    /// composited in its own `render` (design/presentation.md), which
    /// registration order guarantees has already run.
    fn render(&mut self) {
        self.update();
    }
}

impl Handler for Ui {
    fn handle(&mut self, envelope: &Envelope) {
        match Spec::decode(&envelope.payload) {
            Ok(spec) => {
                self.pending
                    .insert(envelope.sender.clone(), PendingSpec::from_message(&spec));
            }
            Err(err) => {
                self.logger.error(
                    "ui",
                    &format!("{} from '{}': {err}", envelope.topic, envelope.sender),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
