use bones_messages::ui::kinds::{
    Button, Checkbox, EmptyState, Heading, Hyperlink, Icon, IconButton, Label, Progress, Select,
    Selectable, Separator, Slider, Space, Spinner, Strong, TextEdit, Toast, Weak,
};
use bones_messages::ui::{Event, Layout, Node, PanelSide, Root, Spec, WidgetProps};
use bones_messages::{DecodeError, Reader, Writer};

use super::{animated_width, draw_spec, transition};
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
        Button::new("b").node(1),
        TextEdit::new("t").node(2),
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
        IconButton::new("download", "Save").primary(true).node(7),
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
fn a_fade_runs_when_it_appears_but_not_when_its_content_changes() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("fade");
    // Draws one pass at `time`, with the fade in it or not.
    let pass = |time: f64, drawn: bool| {
        let mut progress = None;
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if drawn {
                    progress = Some(transition(ui, id));
                }
            });
        });
        progress.unwrap_or(0.0)
    };

    assert!(pass(0.0, true) < 0.01, "first sight fades in");
    assert!(pass(0.1, true) > 0.0);
    assert_eq!(pass(1.0, true), 1.0);
    // Drawn every pass, whatever its content, it stays shown: loading more
    // photos into a faded screen must not fade it again.
    assert_eq!(pass(2.0, true), 1.0);
    pass(3.0, false);
    assert!(pass(4.0, true) < 0.01, "coming back fades in again");
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
    // The tile reports appearing; the stage's bar always shows; a tile's
    // actions only under the pointer.
    assert_eq!(outcome.events[0], Event::Appeared { id: 1 });
    let ids: Vec<u32> = outcome.events[1..].iter().map(Event::id).collect();
    assert_eq!(ids, [2]);
}

#[test]
fn wrap_right_to_left_and_a_modal_draw_their_children() {
    let mut registry = Registry::standard();
    registry.register(GaugeKind);
    let row = |layout, id| Node::container(layout, vec![Gauge { level: id }.node(id)]);
    let screen_spec = screen(vec![row(Layout::Wrap, 1), row(Layout::RightToLeft, 2)]);
    let modal = Spec {
        root: Root::Modal {
            title: "Rename".into(),
        },
        body: row(Layout::Vertical, 3),
    };

    let mut ids: Vec<u32> = run(&screen_spec, &registry)
        .events
        .iter()
        .chain(run(&modal, &registry).events.iter())
        .map(Event::id)
        .collect();
    ids.sort();

    assert_eq!(ids, [1, 2, 3]);
}

#[test]
fn a_gallery_draws_and_announces_only_the_tiles_on_screen() {
    let tiles: Vec<Node> = (1..=60)
        .map(|id| Node::Container {
            id,
            layout: Layout::Tile {
                image: id,
                width: 160,
                height: 100,
                badge: 0,
            },
            children: vec![],
        })
        .collect();
    let spec = screen(vec![Node::Container {
        id: 9,
        layout: Layout::Scroll,
        children: vec![Node::container(
            Layout::Gallery {
                row_height: 100,
                gap: 8,
            },
            tiles,
        )],
    }]);
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 300.0),
        )),
        ..Default::default()
    };
    let mut outcome = None;
    let _ = ctx.run(input, |ctx| {
        outcome = Some(draw_spec(ctx, "owner", &spec, &Registry::standard(), None));
    });

    let appeared: Vec<u32> = outcome
        .unwrap()
        .events
        .iter()
        .filter(|event| matches!(event, Event::Appeared { .. }))
        .map(Event::id)
        .collect();
    assert!(!appeared.is_empty() && appeared.len() < 20, "{appeared:?}");
    assert_eq!(appeared[0], 1);
}

#[test]
fn a_gallery_sees_through_wrappers_to_its_tiles() {
    let wrapped = |id| Node::Container {
        id: 100 + id,
        layout: Layout::ContextMenu,
        children: vec![
            Node::Container {
                id: 200 + id,
                layout: Layout::Draggable,
                children: vec![Node::Container {
                    id,
                    layout: Layout::Tile {
                        image: 0,
                        width: 300,
                        height: 100,
                        badge: 0,
                    },
                    children: vec![],
                }],
            },
            Label {
                text: "menu".into(),
            }
            .node(0),
        ],
    };
    let spec = screen(vec![Node::container(
        Layout::Gallery {
            row_height: 100,
            gap: 8,
        },
        vec![wrapped(1), wrapped(2)],
    )]);

    let outcome = run(&spec, &Registry::standard());

    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    let appeared: Vec<u32> = outcome.events.iter().map(Event::id).collect();
    assert_eq!(appeared, [1, 2]);
}

/// Presses `button` on a draggable label, moves the pointer while holding it,
/// and reports whether a drag-and-drop payload was published.
fn drag_with(button: egui::PointerButton) -> bool {
    let spec = screen(vec![Node::Container {
        id: 7,
        layout: Layout::Draggable,
        children: vec![Label {
            text: "a wide enough label to grab".into(),
        }
        .node(0)],
    }]);
    let registry = Registry::standard();
    let ctx = egui::Context::default();
    let start = egui::pos2(30.0, 15.0);
    let press = |pressed| egui::Event::PointerButton {
        pos: start,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    // Two frames before the press: an item has no rect to drag on its first.
    let frames = [
        vec![egui::Event::PointerMoved(start)],
        vec![],
        vec![press(true)],
        vec![egui::Event::PointerMoved(start + egui::vec2(10.0, 10.0))],
        vec![egui::Event::PointerMoved(start + egui::vec2(40.0, 40.0))],
        vec![],
    ];
    let mut dragged = false;
    for events in frames {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            draw_spec(ctx, "owner", &spec, &registry, None);
        });
        dragged |= egui::DragAndDrop::has_any_payload(&ctx);
    }
    dragged
}

#[test]
fn a_draggable_is_dragged_by_the_primary_button() {
    assert!(drag_with(egui::PointerButton::Primary));
}

#[test]
fn a_right_or_middle_button_does_not_drag_so_the_context_menu_can_open() {
    assert!(!drag_with(egui::PointerButton::Secondary));
    assert!(!drag_with(egui::PointerButton::Middle));
}

/// Runs `spec` through `frames` of input on an 800 × 600 screen and returns
/// every event, whether a drag payload was ever published, and the bounds of
/// the widgets each frame laid out.
fn drive(spec: &Spec, frames: Vec<Vec<egui::Event>>) -> (Vec<Event>, bool, Vec<egui::Rect>) {
    let registry = Registry::standard();
    let ctx = egui::Context::default();
    let (mut events, mut dragged, mut used) = (Vec::new(), false, Vec::new());
    for frame in frames {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events: frame,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            events.extend(draw_spec(ctx, "owner", spec, &registry, None).events);
        });
        dragged |= egui::DragAndDrop::has_any_payload(&ctx);
        used.push(ctx.viewport(|viewport| {
            let mut all = egui::Rect::NOTHING;
            for (_, rects) in viewport.this_pass.widgets.layers() {
                // Panels and the scroll area span the screen, and measuring
                // passes are disabled; the rest is what was laid out.
                for widget in rects.iter().filter(|w| w.enabled && w.rect.width() < 700.0) {
                    all = all.union(widget.rect);
                }
            }
            all
        }));
    }
    (events, dragged, used)
}

fn primary(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn a_click_on_a_button_inside_a_draggable_reaches_the_button() {
    let spec = screen(vec![Node::Container {
        id: 7,
        layout: Layout::Draggable,
        children: vec![Button {
            label: "Apply wallpaper now".into(),
            disabled: false,
        }
        .node(5)],
    }]);
    let at = egui::pos2(30.0, 15.0);
    let (events, dragged, _) = drive(
        &spec,
        vec![
            vec![egui::Event::PointerMoved(at)],
            vec![],
            vec![primary(at, true)],
            vec![primary(at, false)],
            vec![],
        ],
    );
    assert!(!dragged, "the press picked the item up");
    assert!(events.contains(&Event::Clicked { id: 5 }), "{events:?}");
}

#[test]
fn a_primary_drag_still_drops_onto_a_target() {
    let spec = screen(vec![
        Node::Container {
            id: 7,
            layout: Layout::Draggable,
            children: vec![Label {
                text: "a wide enough label to grab".into(),
            }
            .node(0)],
        },
        Space { amount: 200.0 }.node(0),
        Node::Container {
            id: 8,
            layout: Layout::DropTarget,
            children: vec![Label {
                text: "a wide enough drop target".into(),
            }
            .node(0)],
        },
    ]);
    let (from, to) = (egui::pos2(30.0, 15.0), egui::pos2(30.0, 235.0));
    let mut frames = vec![
        vec![egui::Event::PointerMoved(from)],
        vec![],
        vec![primary(from, true)],
    ];
    for step in 1..=10 {
        frames.push(vec![egui::Event::PointerMoved(
            from.lerp(to, step as f32 / 10.0),
        )]);
    }
    frames.push(vec![primary(to, false)]);
    frames.push(vec![]);
    let (events, _, _) = drive(&spec, frames);
    assert!(
        events.contains(&Event::Dropped { id: 8, payload: 7 }),
        "{events:?}"
    );
}

#[test]
fn hovering_a_narrow_tile_with_wide_actions_leaves_the_layout_alone() {
    let spec = screen(vec![Node::Container {
        id: 9,
        layout: Layout::Scroll,
        children: vec![Node::container(
            Layout::Gallery {
                row_height: 100,
                gap: 8,
            },
            vec![Node::Container {
                id: 1,
                layout: Layout::Tile {
                    image: 0,
                    width: 100,
                    height: 100,
                    badge: 0,
                },
                children: vec![Label {
                    text: "a credit far wider than the narrow tile it sits on".into(),
                }
                .node(0)],
            }],
        )],
    }]);
    let over = egui::pos2(50.0, 50.0);
    let mut frames = vec![vec![]];
    // Long enough for the hover fade to reach the actions.
    frames.extend((0..30).map(|_| vec![egui::Event::PointerMoved(over)]));
    let (_, _, used) = drive(&spec, frames);
    let (before, hovered) = (used[1], used[used.len() - 1]);
    // An action that does not fit is left out, so nothing widens what the
    // gallery lays out; spilling past its left edge is what reflowed the rows.
    assert!(
        hovered.left() >= before.left() && hovered.right() <= before.right(),
        "the actions spilled out of the tile: {before:?} became {hovered:?}"
    );
}

/// The width `side_width` gives a side panel holding one label.
fn side_width_for(text: &str) -> f32 {
    let registry = Registry::standard();
    let mut outcome = super::Outcome::default();
    let mut walk = super::Walk {
        registry: &registry,
        images: None,
        tile_size: None,
        outcome: &mut outcome,
    };
    let children = vec![Label { text: text.into() }.node(0)];
    let mut width = 0.0;
    let _ = egui::Context::default().run(egui::RawInput::default(), |ctx| {
        width = walk.side_width(ctx, egui::Id::new("left"), &children);
    });
    width
}

#[test]
fn a_side_panel_fits_its_entries_on_one_line_and_shrinks_with_them() {
    let long = side_width_for("A navigation entry far longer than a default panel");
    let short = side_width_for("B");
    // egui's default side panel is 200 wide and would wrap the long entry.
    assert!(long > 250.0, "{long}");
    // Collapsing to icons must narrow the panel; egui panels only grow.
    assert!(short < 60.0, "{short}");
}

/// How many shapes one pass of `spec` paints.
fn shapes_painted(spec: &Spec) -> usize {
    let ctx = egui::Context::default();
    let registry = Registry::standard();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        draw_spec(ctx, "owner", spec, &registry, None);
    });
    // Placeholders a frame reserved but did not fill paint nothing.
    output
        .shapes
        .iter()
        .filter(|clipped| !matches!(clipped.shape, egui::Shape::Noop))
        .count()
}

#[test]
fn a_drop_target_paints_nothing_until_a_drag_hovers_it() {
    let heading = || {
        Label {
            text: "colorful".into(),
        }
        .node(0)
    };
    let plain = screen(vec![heading()]);
    let target = screen(vec![Node::Container {
        id: 8,
        layout: Layout::DropTarget,
        children: vec![heading()],
    }]);
    // A background behind a heading that takes drops made it look like a
    // different kind of element.
    assert_eq!(shapes_painted(&target), shapes_painted(&plain));
}

#[test]
fn an_icon_only_button_stays_square_in_a_side_panel() {
    let spec = Spec {
        root: Root::Screen,
        body: Node::container(
            Layout::Vertical,
            vec![Node::Container {
                id: 0,
                layout: Layout::Panel {
                    side: PanelSide::Left,
                },
                children: vec![
                    IconButton::new("panel-left-close", "").node(1),
                    Label {
                        text: "A navigation entry that widens the panel".into(),
                    }
                    .node(0),
                ],
            }],
        ),
    };
    let ctx = egui::Context::default();
    let registry = Registry::standard();
    let mut button = None;
    for _ in 0..2 {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_spec(ctx, "owner", &spec, &registry, None);
        });
        button = ctx.viewport(|viewport| {
            viewport
                .this_pass
                .widgets
                .layers()
                .flat_map(|(_, rects)| rects.iter())
                .filter(|w| w.enabled && w.sense.senses_click())
                .map(|w| w.rect)
                .min_by(|a, b| a.top().total_cmp(&b.top()))
        });
    }
    let button = button.expect("the button was drawn");
    // At the top, where it was put: a layout of its own once took all the
    // height and centred it, pushing the entries after it out of the panel.
    assert!(button.top() < 50.0, "the button left the top: {button:?}");
    assert!(
        (button.width() - button.height()).abs() < 2.0,
        "the panel stretched the button: {button:?}"
    );
}

/// Where a button below `progress` lands.
fn top_of_button_below(progress: Progress) -> f32 {
    let spec = screen(vec![
        progress.node(0),
        Button {
            label: "below".into(),
            disabled: false,
        }
        .node(1),
    ]);
    let ctx = egui::Context::default();
    let registry = Registry::standard();
    // The widget list is read after a second pass; the first leaves it empty.
    for _ in 0..2 {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_spec(ctx, "owner", &spec, &registry, None);
        });
    }
    ctx.viewport(|viewport| {
        viewport
            .this_pass
            .widgets
            .layers()
            .flat_map(|(_, rects)| rects.iter())
            .filter(|w| w.enabled && w.sense.senses_click())
            .map(|w| w.rect.top())
            .fold(f32::MIN, f32::max)
    })
}

#[test]
fn a_progress_bar_keeps_its_height_in_every_state() {
    let at = |fraction: f32, text: &str| {
        top_of_button_below(Progress {
            fraction,
            text: text.into(),
        })
    };
    let idle = at(f32::NAN, "");
    // A strip that came and went, or gained a caption, moved the gallery
    // below it up and down while it scrolled.
    assert!(idle > 0.0, "the button below was not found");
    assert_eq!(at(-1.0, ""), idle);
    assert_eq!(at(0.4, "Loading wallpapers 2 / 5"), idle);
}

#[test]
fn a_side_panel_slides_to_a_new_width() {
    let ctx = egui::Context::default();
    let panel = egui::Id::new("left");
    let width_for = |target: f32| {
        let mut width = 0.0;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            width = animated_width(ctx, panel, target);
        });
        width
    };
    assert_eq!(
        width_for(200.0),
        200.0,
        "first sight takes its width at once"
    );
    // Collapsing to icons: the old width on the frame it changes, part of the
    // way on the next, then all of it.
    assert_eq!(width_for(48.0), 200.0);
    let sliding = width_for(48.0);
    assert!(sliding > 48.0 && sliding < 200.0, "it snapped: {sliding}");
    for _ in 0..30 {
        width_for(48.0);
    }
    assert_eq!(width_for(48.0), 48.0);
}

/// Where a button in the central panel starts, one pass with `entry` in the
/// side panel.
fn central_left(ctx: &egui::Context, entry: &str) -> f32 {
    let spec = Spec {
        root: Root::Screen,
        body: Node::container(
            Layout::Vertical,
            vec![
                Node::Container {
                    id: 0,
                    layout: Layout::Panel {
                        side: PanelSide::Left,
                    },
                    children: vec![Label { text: entry.into() }.node(0)],
                },
                Node::Container {
                    id: 0,
                    layout: Layout::Panel {
                        side: PanelSide::Central,
                    },
                    children: vec![Button {
                        label: "photos".into(),
                        disabled: false,
                    }
                    .node(1)],
                },
            ],
        ),
    };
    let registry = Registry::standard();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        draw_spec(ctx, "owner", &spec, &registry, None);
    });
    ctx.viewport(|viewport| {
        viewport
            .this_pass
            .widgets
            .layers()
            .flat_map(|(_, rects)| rects.iter())
            .filter(|w| w.enabled && w.sense.senses_click())
            .map(|w| w.rect.left())
            .fold(f32::MIN, f32::max)
    })
}

#[test]
fn expanding_a_side_panel_moves_the_central_area_smoothly() {
    let ctx = egui::Context::default();
    let long = "A navigation entry far wider than the collapsed panel";
    for _ in 0..3 {
        central_left(&ctx, "B");
    }
    let collapsed = central_left(&ctx, "B");
    let mut lefts = Vec::new();
    for _ in 0..30 {
        lefts.push(central_left(&ctx, long));
    }
    let expanded = *lefts.last().unwrap();
    assert!(expanded > collapsed + 100.0, "{collapsed} → {expanded}");
    // Overflowing entries once widened the panel at once, so the central
    // area jumped to its final place on the first frame.
    let largest_step = lefts
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .chain([lefts[0] - collapsed])
        .fold(0.0, f32::max);
    assert!(
        largest_step < (expanded - collapsed) / 2.0,
        "jumped {largest_step} of {}: {lefts:?}",
        expanded - collapsed
    );
}

/// How many rows `text` took where it was painted in one pass's output; 0
/// when it was not painted.
fn painted_rows(output: &egui::FullOutput, text: &str) -> usize {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text().contains(text) => {
                Some(shape.galley.rows.len())
            }
            _ => None,
        })
        .unwrap_or(0)
}

#[test]
fn hovering_a_collapsed_side_entry_shows_its_name() {
    let spec = Spec {
        root: Root::Screen,
        body: Node::container(
            Layout::Vertical,
            vec![Node::Container {
                id: 0,
                layout: Layout::Panel {
                    side: PanelSide::Left,
                },
                children: vec![Node::container(
                    Layout::Tooltip,
                    vec![
                        // Narrow, as the collapse toggle and collapsed
                        // entries are.
                        IconButton::new("panel-left-close", "").node(1),
                        Label {
                            text: "Library tip".into(),
                        }
                        .node(0),
                    ],
                )],
            }],
        ),
    };
    let registry = Registry::standard();
    let ctx = egui::Context::default();
    let over = egui::pos2(14.0, 14.0);
    let mut rows = 0;
    // Tooltips wait for the pointer to rest, so give it a second of passes.
    for pass in 0..90 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            time: Some(pass as f64 / 60.0),
            // Moved there once, then resting: a move restarts the delay.
            events: if pass < 2 {
                vec![egui::Event::PointerMoved(over)]
            } else {
                vec![]
            },
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            draw_spec(ctx, "owner", &spec, &registry, None);
        });
        rows = rows.max(painted_rows(&output, "Library tip"));
    }
    assert!(rows > 0, "no tooltip with the hidden name");
    // A tip once took the width of the narrow entry and broke into pieces.
    assert_eq!(rows, 1, "the short tip wrapped");
}
