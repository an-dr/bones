use bones_messages::ui::kinds::Label;
use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec, WidgetProps};
use bones_messages::{DecodeError, Reader, Writer};

use super::draw_spec;
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

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Gauge, events: &mut Vec<Event>) {
        ui.label(format!("{}%", props.level));
        events.push(Event::Changed {
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
        outcome = Some(draw_spec(ctx, "owner", spec, registry));
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
