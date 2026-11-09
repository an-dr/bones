use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec};

use crate::draw_context::DrawContext;
use crate::media;
use crate::registry::{DrawProblem, Registry};

/// What drawing one owner's spec produced, besides pixels.
#[derive(Default)]
pub(crate) struct Outcome {
    /// What the user did, for the owner.
    pub(crate) events: Vec<Event>,
    /// Widgets that could not be drawn, by kind name.
    pub(crate) problems: Vec<(String, DrawProblem)>,
}

/// The images one owner uploaded, by handle.
pub(crate) type Images = HashMap<u32, egui::TextureHandle>;

struct Walk<'a> {
    registry: &'a Registry,
    images: Option<&'a Images>,
    outcome: &'a mut Outcome,
}

/// Draws one owner's spec: a floating window, or the whole screen.
pub(crate) fn draw_spec(
    ctx: &egui::Context,
    owner: &str,
    spec: &Spec,
    registry: &Registry,
    images: Option<&Images>,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut walk = Walk {
        registry,
        images,
        outcome: &mut outcome,
    };
    match &spec.root {
        Root::Window { title } => {
            egui::Window::new(title.as_str())
                .id(egui::Id::new(owner))
                .show(ctx, |ui| walk.node(ui, &spec.body));
        }
        Root::Screen => walk.screen(ctx, owner, &spec.body),
    }
    outcome
}

impl Walk<'_> {
    /// Lays a Screen's direct `Panel` children out along the window's edges
    /// (top and bottom span the width, sides fill between them), then puts
    /// the central panel's children and every non-panel child in what is
    /// left. egui needs the edge panels first, whatever the tree's order.
    fn screen(&mut self, ctx: &egui::Context, owner: &str, body: &Node) {
        let children = match body {
            Node::Container { children, .. } => children.as_slice(),
            widget => std::slice::from_ref(widget),
        };
        let panel = |side: PanelSide| {
            children
                .iter()
                .enumerate()
                .filter_map(move |(index, child)| match child {
                    Node::Container {
                        layout: Layout::Panel { side: found },
                        children,
                        ..
                    } if *found == side => Some((index, children.as_slice())),
                    _ => None,
                })
        };
        let id = |side: &str, index: usize| egui::Id::new((owner, side, index));
        for (index, children) in panel(PanelSide::Top) {
            egui::TopBottomPanel::top(id("top", index)).show(ctx, |ui| self.children(ui, children));
        }
        for (index, children) in panel(PanelSide::Bottom) {
            egui::TopBottomPanel::bottom(id("bottom", index))
                .show(ctx, |ui| self.children(ui, children));
        }
        for (index, children) in panel(PanelSide::Left) {
            egui::SidePanel::left(id("left", index))
                .resizable(false)
                .show(ctx, |ui| self.children(ui, children));
        }
        for (index, children) in panel(PanelSide::Right) {
            egui::SidePanel::right(id("right", index))
                .resizable(false)
                .show(ctx, |ui| self.children(ui, children));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            for child in children {
                match child {
                    Node::Container {
                        layout:
                            Layout::Panel {
                                side: PanelSide::Central,
                            },
                        children,
                        ..
                    } => self.children(ui, children),
                    Node::Container {
                        layout: Layout::Panel { .. },
                        ..
                    } => {}
                    other => self.node(ui, other),
                }
            }
        });
    }

    fn node(&mut self, ui: &mut egui::Ui, node: &Node) {
        match node {
            Node::Container {
                id,
                layout,
                children,
            } => self.container(ui, *id, *layout, children),
            Node::Widget { kind, id, props } => {
                let mut cx = DrawContext::new(&mut self.outcome.events, self.images);
                let result = self.registry.draw(ui, kind, *id, props, &mut cx);
                if let Err(problem) = result {
                    self.outcome.problems.push((kind.clone(), problem));
                }
            }
        }
    }

    fn children(&mut self, ui: &mut egui::Ui, children: &[Node]) {
        for child in children {
            self.node(ui, child);
        }
    }

    fn container(&mut self, ui: &mut egui::Ui, id: u32, layout: Layout, children: &[Node]) {
        // Owner ids are per owner; zero means "none", so those take
        // egui's positional id instead of all sharing one.
        let state_id = match id {
            0 => ui.next_auto_id(),
            id => ui.id().with(id),
        };
        match layout {
            Layout::Vertical | Layout::Panel { .. } => {
                ui.vertical(|ui| self.children(ui, children));
            }
            Layout::Horizontal => {
                ui.horizontal(|ui| self.children(ui, children));
            }
            Layout::Centred => {
                ui.vertical_centered(|ui| self.children(ui, children));
            }
            Layout::Scroll => {
                egui::ScrollArea::vertical()
                    .id_salt(state_id)
                    .show(ui, |ui| self.children(ui, children));
            }
            Layout::Frame => {
                // A card: a titled group is a frame whose first child is a
                // `strong` title.
                let visuals = ui.visuals();
                egui::Frame::new()
                    .fill(visuals.faint_bg_color)
                    .corner_radius(visuals.widgets.inactive.corner_radius)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.children(ui, children);
                    });
            }
            Layout::Draggable => {
                let payload = id;
                ui.dnd_drag_source(state_id, payload, |ui| self.children(ui, children));
            }
            Layout::DropTarget => {
                let frame = egui::Frame::new().inner_margin(2);
                let (_, dropped) =
                    ui.dnd_drop_zone::<u32, _>(frame, |ui| self.children(ui, children));
                if let Some(payload) = dropped {
                    self.outcome.events.push(Event::Dropped {
                        id,
                        payload: *payload,
                    });
                }
            }
            Layout::ContextMenu => {
                let Some((area, menu)) = children.split_first() else {
                    return;
                };
                let response = ui
                    .scope(|ui| self.node(ui, area))
                    .response
                    .interact(egui::Sense::click());
                response.context_menu(|ui| self.children(ui, menu));
            }
            Layout::Tooltip => {
                let Some((area, tip)) = children.split_first() else {
                    return;
                };
                let response = ui.scope(|ui| self.node(ui, area)).response;
                response.on_hover_ui(|ui| self.children(ui, tip));
            }
            Layout::Fade => {
                let progress = transition(ui, state_id, children);
                ui.scope(|ui| {
                    ui.set_opacity(progress);
                    ui.add_space((1.0 - progress) * RISE);
                    self.children(ui, children);
                });
            }
            Layout::Tile {
                image,
                width,
                height,
                badge,
            } => {
                let size = egui::vec2(f32::from(width), f32::from(height));
                let texture = self.images.and_then(|images| images.get(&image)).cloned();
                let clicked = media::tile(ui, state_id, size, texture, badge, |ui| {
                    self.children(ui, children)
                });
                if clicked {
                    self.outcome.events.push(Event::Clicked { id });
                }
            }
            Layout::Stage {
                image,
                previous,
                next,
            } => {
                let texture = self.images.and_then(|images| images.get(&image)).cloned();
                let action = media::stage(ui, state_id, texture, previous, next, |ui| {
                    self.children(ui, children)
                });
                match action {
                    Some(media::StageAction::Close) => {
                        self.outcome.events.push(Event::Clicked { id })
                    }
                    Some(media::StageAction::Step(index)) => {
                        self.outcome.events.push(Event::Selected { id, index })
                    }
                    None => {}
                }
            }
            Layout::Grid { columns } => {
                let columns = columns.max(1) as usize;
                egui::Grid::new(state_id).show(ui, |ui| {
                    for (index, child) in children.iter().enumerate() {
                        self.node(ui, child);
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
            }
        }
    }
}

/// How long a `fade` container's change takes, in seconds: short enough that
/// nothing feels like it waits for the animation.
const TRANSITION_SECONDS: f64 = 0.18;

/// How far, in points, new content rises into place.
const RISE: f32 = 8.0;

/// Eased progress from 0 to 1 since `children` last changed under `id`;
/// 1 the first time they are seen.
fn transition(ui: &egui::Ui, id: egui::Id, children: &[Node]) -> f32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    children.hash(&mut hasher);
    let fingerprint = hasher.finish();
    let time = ui.input(|i| i.time);
    let started = ui.ctx().data_mut(|data| {
        let entry = data.get_temp_mut_or_insert_with(id, || (fingerprint, f64::NEG_INFINITY));
        if entry.0 != fingerprint {
            *entry = (fingerprint, time);
        }
        entry.1
    });
    let linear = ((time - started) / TRANSITION_SECONDS).clamp(0.0, 1.0) as f32;
    if linear < 1.0 {
        ui.ctx().request_repaint();
    }
    1.0 - (1.0 - linear).powi(3)
}

#[cfg(test)]
mod tests;
