use bones_messages::ui::kinds::{
    Button, Checkbox, EmptyState, Heading, Hyperlink, Icon, IconButton, Label, Progress, Select,
    Selectable, Separator, Slider, Space, Spinner, Strong, TextEdit, Toast, Weak,
};
use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec, WidgetProps};
use bones_messages::{DecodeError, Reader, Writer};

use super::draw_spec;
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
