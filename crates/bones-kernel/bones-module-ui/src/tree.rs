use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec};

use crate::registry::{DrawProblem, Registry};

/// What drawing one owner's spec produced, besides pixels.
#[derive(Default)]
pub(crate) struct Outcome {
    /// What the user did, for the owner.
    pub(crate) events: Vec<Event>,
    /// Widgets that could not be drawn, by kind name.
    pub(crate) problems: Vec<(String, DrawProblem)>,
}

struct Walk<'a> {
    registry: &'a Registry,
    outcome: &'a mut Outcome,
}

/// Draws one owner's spec: a floating window, or the whole screen.
pub(crate) fn draw_spec(
    ctx: &egui::Context,
    owner: &str,
    spec: &Spec,
    registry: &Registry,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut walk = Walk {
        registry,
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
                let result = self
                    .registry
                    .draw(ui, kind, *id, props, &mut self.outcome.events);
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
                ui.group(|ui| self.children(ui, children));
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

#[cfg(test)]
mod tests;
