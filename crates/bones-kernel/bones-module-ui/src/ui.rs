use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel_infra::draw_target::{DrawTargetService, UiMesh, UiVertex};
use bones_kernel_infra::logging::Logger;
use bones_kernel_infra::ui_port::UiService;
use bones_messages::ui::{FreeImage, Root, SetImage, Spec};
use bones_messages::{DecodeMessage, Message};

use crate::display_scale::display_scale;
use crate::fonts;
use crate::input_translation::{compute_modifiers, translate_key, translate_mouse_button};
use crate::kind::Kind;
use crate::look::NativeLook;
use crate::native_door::{upsert, ImageOp, NativeDoor};
use crate::output_translation::{compute_texture_key, convert_color_image_to_straight_rgba};
use crate::platform_output;
use crate::registry::{DrawProblem, Registry};
use crate::tree::{draw_spec, Images, Outcome};

/// Owns the embedded egui context: decodes widget specs published on
/// `ui/spec`, runs one egui frame per tick, and submits the resulting
/// meshes to whatever surface provided `draw-target`.
pub struct Ui {
    ctx: egui::Context,
    bus: Bus,
    // Keyed by the publishing extension's bus sender name; ADR-005 is
    // immediate-mode, so a spec not republished this frame draws nothing —
    // `update` drains this on every call.
    pending: Vec<(String, Spec)>,
    registry: Registry,
    // The desktop's look, when the composition asked for it (ADR-049).
    look: Option<NativeLook>,
    // Uploaded images by owner, and uploads from extensions waiting for the
    // next frame (native ones wait in the door).
    images: HashMap<String, Images>,
    image_ops: Vec<(String, ImageOp)>,
    // Trees native modules showed through the `ui` service, and the events
    // waiting for them; shared with every module that resolved the service.
    door: Arc<Mutex<NativeDoor>>,
    // (owner, kind) pairs already reported, so a broken widget republished
    // every frame logs one line.
    reported: HashSet<(String, String)>,
    // The one owner whose `Screen` tree is drawn; kept until it stops
    // showing one, so a second claimant cannot take the window mid-use.
    screen_owner: Option<String>,
    // Physical pixels per point: the display's scale times the zoom a
    // native owner set. Pointer positions and output are converted with it.
    scale: f32,
    display: f32,
    scale_checked: Option<Instant>,
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
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts::definitions(None));
        Self {
            ctx,
            bus,
            pending: Vec::new(),
            registry: Registry::standard(),
            look: None,
            images: HashMap::new(),
            image_ops: Vec::new(),
            door: Arc::default(),
            reported: HashSet::new(),
            screen_owner: None,
            scale: 1.0,
            display: 1.0,
            scale_checked: None,
            events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            pointer_pos: egui::Pos2::ZERO,
            start: Instant::now(),
            logger,
            target: None,
        }
    }

    /// Follows the desktop's font, mode, accent, and spacing from now on
    /// (ADR-049). The kernel builder calls it unless the composition opts
    /// out; without it egui's own look stays.
    pub fn use_native_look(&mut self) {
        if self.look.is_none() {
            self.look = Some(NativeLook::start(&self.ctx));
        }
    }

    /// Adds a widget kind every owner's tree can place (ADR-048); a kind
    /// with the name of an existing one replaces it.
    pub fn register<K: Kind>(&mut self, kind: K) {
        self.registry.register(kind);
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
                self.pointer_pos = egui::pos2(*x, *y) / self.scale;
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
                self.pointer_pos = egui::pos2(*x, *y) / self.scale;
                self.events.push(egui::Event::PointerButton {
                    pos: self.pointer_pos,
                    button,
                    pressed: matches!(event, E::MouseButtonDown { .. }),
                    modifiers: self.modifiers,
                });
                self.wants_pointer_input()
            }
            E::MouseWheel { x, y, .. } => {
                self.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(*x, *y),
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
                let pressed = matches!(event, E::KeyDown { .. });
                // egui reads the clipboard only through these events.
                if pressed && self.modifiers.command {
                    match mapped {
                        egui::Key::V => {
                            if let Some(text) = platform_output::clipboard_text() {
                                self.events.push(egui::Event::Paste(text));
                            }
                        }
                        egui::Key::C => self.events.push(egui::Event::Copy),
                        egui::Key::X => self.events.push(egui::Event::Cut),
                        _ => {}
                    }
                }
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
        self.refresh_scale();
        self.apply_image_ops();
        let screen_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(screen_width as f32, screen_height as f32) / self.scale,
        );
        let mut raw_input = egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(self.start.elapsed().as_secs_f64()),
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.scale);

        // Extension trees and native trees draw alike; only where their
        // events go differs.
        let native = std::mem::take(&mut self.door.lock().unwrap().shown);
        let pending: Vec<(String, Spec, bool)> = std::mem::take(&mut self.pending)
            .into_iter()
            .map(|(owner, spec)| (owner, spec, false))
            .chain(native.into_iter().map(|(owner, spec)| (owner, spec, true)))
            .collect();
        let pending = self.one_screen(pending);
        let time = self.start.elapsed().as_secs_f64();
        let theme = self.door.lock().unwrap().theme;
        if let Some(look) = &mut self.look {
            look.follow(&self.ctx, theme, time);
        }
        let registry = &self.registry;
        let images = &self.images;
        let look = &mut self.look;
        let mut outcomes: Vec<Outcome> = Vec::new();
        let full_output = self.ctx.run(raw_input, |ctx| {
            // egui may run the closure more than once per frame; only the
            // last pass's outcome is the one the user saw.
            outcomes = pending
                .iter()
                .map(|(owner, spec, _)| draw_spec(ctx, owner, spec, registry, images.get(owner)))
                .collect();
            if let Some(look) = look.as_mut() {
                look.paint_fade(ctx, time);
            }
        });
        for ((owner, _, native), outcome) in pending.iter().zip(outcomes) {
            self.report(owner, *native, outcome);
        }

        platform_output::apply(&full_output.platform_output, &self.logger);
        let ppp = full_output.pixels_per_point;
        let clipped = self.ctx.tessellate(full_output.shapes, ppp);

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
                            x: vertex.pos.x * ppp,
                            y: vertex.pos.y * ppp,
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
                    (clip.min.x * ppp).round() as i32,
                    (clip.min.y * ppp).round() as i32,
                    (clip.width() * ppp).round().max(0.0) as u32,
                    (clip.height() * ppp).round().max(0.0) as u32,
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

impl Ui {
    /// How often the display's scale is asked again: it changes only when
    /// the window moves to another display or the user changes the setting.
    const SCALE_POLL: Duration = Duration::from_secs(1);

    /// Uploads and frees the images both doors queued since the last frame.
    fn apply_image_ops(&mut self) {
        let native = std::mem::take(&mut self.door.lock().unwrap().images);
        let ops: Vec<(String, ImageOp)> = std::mem::take(&mut self.image_ops)
            .into_iter()
            .chain(native)
            .collect();
        for (owner, op) in ops {
            match op {
                ImageOp::Set {
                    handle,
                    width,
                    height,
                    rgba,
                } => {
                    let size = [width as usize, height as usize];
                    if rgba.len() != size[0] * size[1] * 4 {
                        let message = format!(
                            "'{owner}' uploaded image {handle} as {width}x{height} with {} bytes; ignored",
                            rgba.len()
                        );
                        self.logger.warn("ui", &message);
                        continue;
                    }
                    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                    let name = format!("{owner}/{handle}");
                    let texture = self
                        .ctx
                        .load_texture(name, pixels, egui::TextureOptions::LINEAR);
                    self.images
                        .entry(owner)
                        .or_default()
                        .insert(handle, texture);
                }
                ImageOp::Free { handle } => {
                    if let Some(images) = self.images.get_mut(&owner) {
                        images.remove(&handle);
                    }
                }
            }
        }
    }

    fn refresh_scale(&mut self) {
        let due = self
            .scale_checked
            .is_none_or(|checked| checked.elapsed() >= Self::SCALE_POLL);
        let display = if due {
            self.scale_checked = Some(Instant::now());
            Some(display_scale())
        } else {
            None
        };
        let zoom = self.door.lock().unwrap().zoom;
        if let Some(display) = display {
            self.display = display;
        }
        self.scale = (self.display * zoom).clamp(0.5, 6.0);
    }

    /// Keeps at most one `Screen` tree: the current holder's while it still
    /// shows one, else the first claimant's. Each refused claimant is told
    /// once who holds the window; its `Window` trees are unaffected.
    fn one_screen(&mut self, pending: Vec<(String, Spec, bool)>) -> Vec<(String, Spec, bool)> {
        let claimants: Vec<&String> = pending
            .iter()
            .filter(|(_, spec, _)| spec.root == Root::Screen)
            .map(|(owner, _, _)| owner)
            .collect();
        let holder = match &self.screen_owner {
            Some(holder) if claimants.contains(&holder) => Some(holder.clone()),
            _ => claimants.first().map(|owner| (*owner).clone()),
        };
        self.screen_owner = holder.clone();
        let mut kept = Vec::with_capacity(pending.len());
        for (owner, spec, native) in pending {
            if spec.root == Root::Screen && Some(&owner) != holder.as_ref() {
                let holder = holder.as_deref().unwrap_or_default();
                if self.reported.insert((owner.clone(), "screen".into())) {
                    self.logger.warn(
                        "ui",
                        &format!(
                            "'{owner}' showed a Screen, but '{holder}' holds the window; not drawn"
                        ),
                    );
                }
                continue;
            }
            kept.push((owner, spec, native));
        }
        kept
    }

    /// Hands a native owner its events, publishes an extension owner's, and
    /// logs each broken widget once per owner and kind.
    fn report(&mut self, owner: &str, native: bool, outcome: Outcome) {
        if native {
            self.door
                .lock()
                .unwrap()
                .events
                .entry(owner.to_string())
                .or_default()
                .extend(outcome.events);
            self.report_problems(owner, outcome.problems);
            return;
        }
        for event in outcome.events {
            let (topic, payload) = event.to_message();
            self.bus.publish(Envelope {
                topic: topic.to_string(),
                sender: "ui".to_string(),
                correlation: None,
                payload,
            });
        }
        self.report_problems(owner, outcome.problems);
    }

    fn report_problems(&mut self, owner: &str, problems: Vec<(String, DrawProblem)>) {
        for (kind, problem) in problems {
            if !self.reported.insert((owner.to_string(), kind.clone())) {
                continue;
            }
            let why = match problem {
                DrawProblem::UnknownKind => "is not a registered widget kind".to_string(),
                DrawProblem::BadProps(error) => format!("has malformed properties: {error}"),
            };
            self.logger.warn(
                "ui",
                &format!("'{owner}' placed a '{kind}' widget, which {why}; drawn as nothing"),
            );
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
    /// to draw on without one. Provides the `ui` service, the native door.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        self.target = Some(ctx.consume_service::<DrawTargetService>().ok_or_else(|| {
            "ui needs a draw-target service (configure .renderer() too)".to_string()
        })?);
        ctx.subscribe(Spec::TOPIC);
        ctx.subscribe(SetImage::TOPIC);
        ctx.subscribe(FreeImage::TOPIC);
        let service: UiService = self.door.clone();
        ctx.provide_service(service)?;
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
        let owner = envelope.sender.clone();
        let decoded = match envelope.topic.as_str() {
            SetImage::TOPIC => SetImage::decode(&envelope.payload).map(|image| {
                let op = ImageOp::Set {
                    handle: image.handle,
                    width: image.width,
                    height: image.height,
                    rgba: image.rgba.to_vec(),
                };
                self.image_ops.push((owner, op));
            }),
            FreeImage::TOPIC => FreeImage::decode(&envelope.payload).map(|free| {
                let op = ImageOp::Free {
                    handle: free.handle,
                };
                self.image_ops.push((owner, op));
            }),
            _ => Spec::decode(&envelope.payload).map(|spec| {
                upsert(&mut self.pending, &owner, spec);
            }),
        };
        match decoded {
            Ok(()) => {}
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
