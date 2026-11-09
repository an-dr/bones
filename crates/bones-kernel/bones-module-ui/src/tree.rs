use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bones_messages::ui::kinds::Label;
use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec, WidgetProps};

use crate::draw_context::DrawContext;
use crate::gallery;
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
    /// The size a gallery gave the item being drawn; the first tile inside
    /// it takes this instead of its own.
    tile_size: Option<egui::Vec2>,
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
        tile_size: None,
        outcome: &mut outcome,
    };
    match &spec.root {
        Root::Window { title } => {
            egui::Window::new(title.as_str())
                .id(egui::Id::new(owner))
                .show(ctx, |ui| walk.node(ui, &spec.body));
        }
        Root::Screen => walk.screen(ctx, owner, &spec.body),
        Root::Modal { title } => {
            egui::Modal::new(egui::Id::new((owner, "modal"))).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.heading(title.as_str());
                ui.add_space(6.0);
                walk.node(ui, &spec.body);
            });
        }
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

    #[allow(clippy::too_many_arguments)]
    fn tile(
        &mut self,
        ui: &mut egui::Ui,
        id: u32,
        state_id: egui::Id,
        image: u32,
        size: egui::Vec2,
        badge: u32,
        children: &[Node],
    ) {
        let texture = self.images.and_then(|images| images.get(&image)).cloned();
        let shown = media::tile(ui, state_id, size, texture, badge, |ui| {
            self.children(ui, children)
        });
        if shown.appeared {
            self.outcome.events.push(Event::Appeared { id });
        }
        if shown.clicked {
            self.outcome.events.push(Event::Clicked { id });
        }
    }

    /// Lays `children` out in justified rows and draws the rows on screen;
    /// the rest only take their room.
    fn gallery(&mut self, ui: &mut egui::Ui, row_height: f32, gap: f32, children: &[Node]) {
        let aspects: Vec<f32> = children
            .iter()
            .map(|child| match inner_tile(child) {
                Some((width, height)) => gallery::aspect(f32::from(width), f32::from(height)),
                None => gallery::DEFAULT_ASPECT,
            })
            .collect();
        let width = ui.available_width();
        for row in gallery::rows(&aspects, width, row_height.max(1.0), gap) {
            let (row_rect, _) =
                ui.allocate_exact_size(egui::vec2(width, row.height), egui::Sense::hover());
            if ui.is_rect_visible(row_rect) {
                let mut x = row_rect.left();
                for (index, item_width) in row.items {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(x, row_rect.top()),
                        egui::vec2(item_width, row.height),
                    );
                    self.gallery_item(ui, &children[index], rect);
                    x += item_width + gap;
                }
            }
            ui.add_space(gap - ui.spacing().item_spacing.y);
        }
    }

    /// One gallery child in `rect`; the tile in it takes the rect's size.
    fn gallery_item(&mut self, ui: &mut egui::Ui, child: &Node, rect: egui::Rect) {
        let builder = egui::UiBuilder::new().max_rect(rect);
        ui.scope_builder(builder, |ui| {
            self.tile_size = Some(rect.size());
            self.node(ui, child);
            self.tile_size = None;
        });
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
                let scroll = egui::ScrollArea::vertical()
                    .id_salt(state_id)
                    .auto_shrink(false)
                    .show(ui, |ui| self.children(ui, children));
                let bottom = scroll.state.offset.y + scroll.inner_rect.height();
                let content = scroll.content_size.y;
                if id != 0 && reached_end(ui, state_id, bottom, content) {
                    self.outcome.events.push(Event::ReachedEnd { id });
                }
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
                response.context_menu(|ui| {
                    let before = self.outcome.events.len();
                    self.children(ui, menu);
                    // An item acted on closes the menu, as menus do.
                    if self.outcome.events.len() > before {
                        ui.close();
                    }
                });
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
                let own = egui::vec2(f32::from(width), f32::from(height));
                let size = self.tile_size.take().unwrap_or(own);
                self.tile(ui, id, state_id, image, size, badge, children);
            }
            Layout::Gallery { row_height, gap } => {
                self.gallery(ui, f32::from(row_height), f32::from(gap), children);
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
            Layout::Wrap => {
                ui.horizontal_wrapped(|ui| self.children(ui, children));
            }
            Layout::RightToLeft => {
                let layout = egui::Layout::right_to_left(egui::Align::Center);
                ui.with_layout(layout, |ui| self.children(ui, children));
            }
            Layout::Submenu => {
                let Some((name, items)) = children.split_first() else {
                    return;
                };
                let title = match name {
                    Node::Widget { kind, props, .. } if kind == Label::KIND => Label::decode(props)
                        .map(|label| label.text)
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                ui.menu_button(title, |ui| self.children(ui, items));
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

/// The width and height of the tile `node` is or wraps: a gallery child may
/// be a tile inside a `draggable`, `context-menu`, or `tooltip`, whose first
/// child is what is drawn.
fn inner_tile(node: &Node) -> Option<(u16, u16)> {
    match node {
        Node::Container {
            layout: Layout::Tile { width, height, .. },
            ..
        } => Some((*width, *height)),
        Node::Container {
            layout: Layout::Draggable | Layout::ContextMenu | Layout::Tooltip | Layout::DropTarget,
            children,
            ..
        } => children.first().and_then(inner_tile),
        _ => None,
    }
}

/// How close to the end, in points, a scroll area reports reaching it, so
/// more can load before the user sees a gap.
const END_MARGIN: f32 = 500.0;

/// Whether a scroll area has come near its end since its content last
/// grew: true once per growth.
fn reached_end(ui: &egui::Ui, id: egui::Id, bottom: f32, content: f32) -> bool {
    if bottom + END_MARGIN < content {
        return false;
    }
    let key = id.with("reported-end");
    let reported = ui.ctx().data(|data| data.get_temp::<f32>(key));
    if reported == Some(content) {
        return false;
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, content));
    true
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
