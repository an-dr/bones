use bones_messages::ui::kinds::{
    Button, Checkbox, EmptyState, Heading, Hyperlink, Icon, IconButton, Label, Progress, Select,
    Selectable, Separator, Slider, Space, Spinner, Strong, TextEdit, Toast, Weak,
};
use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec, WidgetProps};
use bones_messages::{DecodeError, Reader, Writer};

use super::{draw_spec, transition};
use crate::draw_context::DrawContext;
use crate::kind::Kind;
use crate::registry::{DrawProblem, Registry};

/// A custom kind as an embedder writes one: its own props, its own event.
#[derive(Debug, PartialEq)]
struct Gauge {
    level: u32,
}

impl WidgetProps for Gauge {
    const KIND: &'static str = "test/gauge";

    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.level).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let level = reader.read_u32()?;
        reader.finish()?;
        Ok(Self { level })
    }
}

struct GaugeKind;

impl Kind for GaugeKind {
    type Props = Gauge;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Gauge, cx: &mut DrawContext<'_>) {
        ui.label(format!("{}%", props.level));
        cx.emit(Event::Changed {
            id,
            text: props.level.to_string(),
        });
    }
}

fn screen(children: Vec<Node>) -> Spec {
    Spec {
        root: Root::Screen,
        body: Node::container(Layout::Vertical, children),
    }
}

fn run(spec: &Spec, registry: &Registry) -> super::Outcome {
    let ctx = egui::Context::default();
    let mut outcome = None;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        outcome = Some(draw_spec(ctx, "owner", spec, registry, None));
    });
    outcome.unwrap()
}

#[test]
fn a_registered_custom_kind_draws_and_reports_events() {
    let mut registry = Registry::standard();
    registry.register(GaugeKind);
    let spec = screen(vec![
        Label { text: "x".into() }.node(0),
        Gauge { level: 40 }.node(5),
    ]);

    let outcome = run(&spec, &registry);

    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    assert_eq!(
        outcome.events,
        [Event::Changed {
            id: 5,
            text: "40".into()
        }]
    );
}

#[test]
fn an_unregistered_kind_is_a_problem_not_a_crash() {
    let spec = screen(vec![Gauge { level: 1 }.node(1)]);

    let outcome = run(&spec, &Registry::standard());

    assert_eq!(
        outcome.problems,
        [("test/gauge".to_string(), DrawProblem::UnknownKind)]
    );
}

#[test]
fn malformed_props_are_a_problem_naming_the_kind() {
    let spec = screen(vec![Node::Widget {
        kind: Label::KIND.into(),
        id: 0,
        props: vec![9],
    }]);

    let outcome = run(&spec, &Registry::standard());

    assert_eq!(
        outcome.problems,
        [(
            "label".to_string(),
            DrawProblem::BadProps(DecodeError::Truncated)
        )]
    );
}

#[test]
fn every_screen_panel_and_loose_child_is_drawn() {
    let mut registry = Registry::standard();
    registry.register(GaugeKind);
    let panel = |side, level| Node::Container {
        id: 0,
        layout: Layout::Panel { side },
        children: vec![Gauge { level }.node(level)],
    };
    // Central first and a loose widget last: order in the tree must not
    // matter, since egui needs edge panels before the central one.
    let spec = screen(vec![
        panel(PanelSide::Central, 5),
        panel(PanelSide::Top, 1),
        panel(PanelSide::Bottom, 2),
        panel(PanelSide::Left, 3),
        panel(PanelSide::Right, 4),
        Gauge { level: 6 }.node(6),
    ]);

    let outcome = run(&spec, &registry);

    let mut ids: Vec<u32> = outcome.events.iter().map(Event::id).collect();
    ids.sort();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn every_standard_kind_is_registered_and_draws() {
    let spec = screen(vec![
        Label { text: "l".into() }.node(0),
        Heading { text: "h".into() }.node(0),
        Weak { text: "w".into() }.node(0),
        Strong { text: "s".into() }.node(0),
        Button { label: "b".into() }.node(1),
        TextEdit { text: "t".into() }.node(2),
        Selectable {
            label: "sel".into(),
            selected: true,
        }
        .node(3),
        Select {
            selected: Some(0),
            placeholder: "p".into(),
            options: vec!["a".into(), "b".into()],
        }
        .node(4),
        Checkbox {
            label: "c".into(),
            checked: true,
        }
        .node(5),
        Slider {
            value: 1.0,
            min: 0.0,
            max: 2.0,
        }
        .node(6),
        Separator.node(0),
        Hyperlink {
            label: "link".into(),
            url: "https://example.com".into(),
        }
        .node(0),
        Spinner.node(0),
        Space { amount: 4.0 }.node(0),
        EmptyState {
            icon: "image".into(),
            title: "t".into(),
            detail: "d".into(),
        }
        .node(0),
        Progress {
            fraction: -1.0,
            text: "c".into(),
        }
        .node(0),
        IconButton {
            icon: "download".into(),
            label: "Save".into(),
            primary: true,
        }
        .node(7),
        Toast { text: "hi".into() }.node(8),
        Icon {
            name: "trash-2".into(),
            size: 0.0,
        }
        .node(0),
    ]);

    let outcome = run(&spec, &Registry::standard());

    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    assert!(outcome.events.is_empty(), "nothing was clicked");
}

#[test]
fn interaction_layouts_draw_their_area() {
    let mut registry = Registry::standard();
    registry.register(GaugeKind);
    let wrap = |layout, id| Node::Container {
        id,
        layout,
        children: vec![
            Gauge { level: id }.node(id),
            Label {
                text: "menu or tip".into(),
            }
            .node(0),
        ],
    };
    let spec = screen(vec![
        wrap(Layout::Draggable, 1),
        wrap(Layout::DropTarget, 2),
        wrap(Layout::ContextMenu, 3),
        wrap(Layout::Tooltip, 4),
    ]);

    let outcome = run(&spec, &registry);

    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    let mut ids: Vec<u32> = outcome.events.iter().map(Event::id).collect();
    ids.sort();
    // Each area is drawn; the menu and tip stay closed without a pointer.
    assert_eq!(ids, [1, 2, 3, 4]);
}

#[test]
fn a_fade_runs_when_its_content_changes_but_not_on_first_sight() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("fade");
    let progress_at = |time: f64, text: &str| {
        let children = vec![Label { text: text.into() }.node(0)];
        let mut progress = 0.0;
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                progress = transition(ui, id, &children);
            });
        });
        progress
    };

    assert_eq!(progress_at(0.0, "a"), 1.0, "first sight shows at once");
    assert_eq!(progress_at(1.0, "a"), 1.0, "unchanged content stays");
    let started = progress_at(2.0, "b");
    let halfway = progress_at(2.09, "b");
    assert!(started < 0.01, "{started}");
    assert!(halfway > 0.0 && halfway < 1.0, "{halfway}");
    assert_eq!(progress_at(2.5, "b"), 1.0);
}

#[test]
fn a_tile_and_a_stage_draw_with_or_without_their_image() {
    let mut registry = Registry::standard();
    registry.register(GaugeKind);
    let spec = screen(vec![
        Node::Container {
            id: 1,
            layout: Layout::Tile {
                image: 9,
                width: 120,
                height: 80,
                badge: 0x2665,
            },
            children: vec![Gauge { level: 1 }.node(1)],
        },
        Node::Container {
            id: 2,
            layout: Layout::Stage {
                image: 9,
                previous: true,
                next: true,
            },
            children: vec![Gauge { level: 2 }.node(2)],
        },
    ]);

    let outcome = run(&spec, &registry);

    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    // The stage's bar always shows; a tile's actions only under the pointer.
    let ids: Vec<u32> = outcome.events.iter().map(Event::id).collect();
    assert_eq!(ids, [2]);
}
