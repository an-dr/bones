use std::collections::HashMap;

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
            let panel_id = id("left", index);
            let width = animated_width(ctx, panel_id, self.side_width(ctx, panel_id, children));
            egui::SidePanel::left(panel_id)
                .resizable(false)
                .exact_width(width)
                .show(ctx, |ui| self.side_children(ui, children));
        }
        for (index, children) in panel(PanelSide::Right) {
            let panel_id = id("right", index);
            let width = animated_width(ctx, panel_id, self.side_width(ctx, panel_id, children));
            egui::SidePanel::right(panel_id)
                .resizable(false)
                .exact_width(width)
                .show(ctx, |ui| self.side_children(ui, children));
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
            self.fitting_children(ui, children)
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

    /// egui's `dnd_drag_source`, except that only the primary button drags
    /// and only once the pointer has moved: egui starts on any button's press,
    /// so a right-click picked the item up instead of opening its menu, and a
    /// press on a button inside it, such as Apply, moved it instead.
    fn draggable(
        &mut self,
        ui: &mut egui::Ui,
        state_id: egui::Id,
        payload: u32,
        children: &[Node],
    ) {
        let ctx = ui.ctx().clone();
        let dragging = ctx.is_being_dragged(state_id)
            && ctx.input(|i| {
                i.pointer.primary_down()
                    && !i.pointer.secondary_down()
                    && !i.pointer.middle_down()
                    && i.pointer.is_decidedly_dragging()
            });
        if !dragging {
            // Sensed below the children, over the rect they took last frame,
            // so a button inside keeps its clicks. A new item cannot be
            // dragged on its first frame, before it has a rect.
            let area = state_id.with("area");
            let rect = ctx.data(|data| data.get_temp::<egui::Rect>(area));
            ui.interact(
                rect.unwrap_or(egui::Rect::NOTHING),
                state_id,
                egui::Sense::drag(),
            );
            let response = ui
                .scope(|ui| {
                    // Text inside would take the drag to select itself.
                    ui.style_mut().interaction.selectable_labels = false;
                    self.children(ui, children)
                })
                .response;
            ctx.data_mut(|data| data.insert_temp(area, response.rect));
            return;
        }
        egui::DragAndDrop::set_payload(&ctx, payload);
        // Still sensed, or egui drops a drag whose widget disappeared.
        let area = state_id.with("area");
        let rect = ctx.data(|data| data.get_temp::<egui::Rect>(area));
        ui.interact(
            rect.unwrap_or(egui::Rect::NOTHING),
            state_id,
            egui::Sense::drag(),
        );
        // Painted on a top layer and moved under the pointer, as egui does.
        let layer = egui::LayerId::new(egui::Order::Tooltip, state_id);
        let response = ui
            .scope_builder(egui::UiBuilder::new().layer_id(layer), |ui| {
                self.children(ui, children)
            })
            .response;
        if let Some(pointer) = ctx.pointer_interact_pos() {
            let delta = pointer - response.rect.center();
            ctx.transform_layer_shapes(layer, egui::emath::TSTransform::from_translation(delta));
        }
    }

    /// The width a side panel's children take on one line, measured in an
    /// invisible pass: egui panels only ever grow, so a panel whose content
    /// shrinks, such as navigation collapsing to icons, would keep its width.
    fn side_width(&mut self, ctx: &egui::Context, panel: egui::Id, children: &[Node]) -> f32 {
        let (events, problems) = (self.outcome.events.len(), self.outcome.problems.len());
        let mut ui = egui::Ui::new(
            ctx.clone(),
            panel.with("measure"),
            egui::UiBuilder::new()
                .max_rect(ctx.screen_rect())
                .sizing_pass()
                .invisible(),
        );
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        self.children(&mut ui, children);
        // The pass only measures: whatever it reported is reported again
        // by the real one.
        self.outcome.events.truncate(events);
        self.outcome.problems.truncate(problems);
        let margin = egui::Frame::side_top_panel(&ctx.style())
            .total_margin()
            .sum()
            .x;
        ui.min_rect().width() + margin
    }

    /// A side panel's children, one per line and as wide as the panel, so
    /// entries line up instead of each taking its own width.
    fn side_children(&mut self, ui: &mut egui::Ui, children: &[Node]) {
        // A child the panel does not allocate, clipped to the panel's width
        // of this frame: egui widens a panel to anything that overflows it, so
        // entries already at their full width would end a slide at once.
        let rect = ui.available_rect_before_wrap();
        let layout = egui::Layout::top_down_justified(egui::Align::Min);
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
        inner.shrink_clip_rect(rect);
        inner.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        self.children(&mut inner, children);
    }

    /// Draws `children` in order while they fit the space left, and stops at
    /// the first that does not: a tile's actions come rightmost first, so a
    /// narrow tile drops its credit rather than cutting it to an ellipsis.
    fn fitting_children(&mut self, ui: &mut egui::Ui, children: &[Node]) {
        let spacing = ui.spacing().item_spacing.x;
        let mut left = ui.available_width();
        for (index, child) in children.iter().enumerate() {
            let (events, problems) = (self.outcome.events.len(), self.outcome.problems.len());
            let mut probe = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("fit", index))
                    .sizing_pass()
                    .invisible(),
            );
            self.node(&mut probe, child);
            self.outcome.events.truncate(events);
            self.outcome.problems.truncate(problems);
            let width = probe.min_rect().width();
            if width > left {
                break;
            }
            left -= width + spacing;
            self.node(ui, child);
        }
    }

    /// `children` as a place to drop a draggable, returning what was dropped.
    /// Unlike egui's drop zone, which always paints a background, it shows
    /// only while a drag hovers it, so a heading that takes drops still looks
    /// like a heading.
    fn drop_target(&mut self, ui: &mut egui::Ui, children: &[Node]) -> Option<u32> {
        let mut frame = egui::Frame::new().inner_margin(2).begin(ui);
        self.children(&mut frame.content_ui, children);
        let response = frame.allocate_space(ui);
        let carrying = egui::DragAndDrop::has_payload_of_type::<u32>(ui.ctx());
        if carrying && response.contains_pointer() {
            let accent = ui.visuals().selection.bg_fill;
            frame.frame.fill = accent.gamma_multiply(0.25);
            frame.frame.stroke = egui::Stroke::new(1.0_f32, accent);
            frame.frame.corner_radius = ui.visuals().widgets.inactive.corner_radius;
            frame.paint(ui);
        }
        response
            .dnd_release_payload::<u32>()
            .map(|payload| *payload)
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
            Layout::Draggable => self.draggable(ui, state_id, id, children),
            Layout::DropTarget => {
                if let Some(payload) = self.drop_target(ui, children) {
                    self.outcome.events.push(Event::Dropped { id, payload });
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
                let rect = ui.scope(|ui| self.node(ui, area)).response.rect;
                // A hover-only area of its own over what was drawn: the
                // scope's own response does not count as hovered while a
                // clickable widget inside it is, so the tip never showed.
                // Sensing only hover, it leaves clicks to that widget.
                ui.interact(rect, state_id.with("tip"), egui::Sense::hover())
                    .on_hover_ui(|ui| {
                        // On one line: egui sizes a tip from too little room
                        // beside a narrow widget, and "Collapse" broke in two.
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        self.children(ui, tip)
                    });
            }
            Layout::Fade => {
                let progress = transition(ui, state_id);
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

/// Eases a side panel toward `target`, so collapsing or expanding it, or an
/// entry growing, slides instead of snapping; the first sight takes it at once.
fn animated_width(ctx: &egui::Context, panel: egui::Id, target: f32) -> f32 {
    ctx.animate_value_with_time(panel.with("width"), target, PANEL_SECONDS)
}

/// How long a side panel takes to reach a new width.
const PANEL_SECONDS: f32 = 0.1;

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
fn transition(ui: &egui::Ui, id: egui::Id) -> f32 {
    // Restarts when the node was not drawn on the previous pass: it just
    // appeared, or came back. What is inside may change freely meanwhile,
    // such as photos loading, without the whole area fading again.
    let pass = ui.ctx().cumulative_pass_nr();
    let time = ui.input(|i| i.time);
    let started = ui.ctx().data_mut(|data| {
        let entry = data.get_temp_mut_or_insert_with(id, || (0u64, time));
        if entry.0 + 1 < pass {
            entry.1 = time;
        }
        entry.0 = pass;
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
