use super::kinds::{Button, Label, TextEdit};
use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Writer};

fn notes() -> Spec {
    Spec {
        root: Root::Window {
            title: "notes".into(),
        },
        body: Node::container(
            Layout::Vertical,
            vec![
                TextEdit {
                    text: "buy milk".into(),
                }
                .node(1),
                Node::container(
                    Layout::Horizontal,
                    vec![Button {
                        label: "Add".into(),
                    }
                    .node(2)],
                ),
                Label {
                    text: "existing note".into(),
                }
                .node(0),
            ],
        ),
    }
}

#[test]
fn nested_window_spec_round_trips() {
    let spec = notes();
    assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
}

#[test]
fn screen_with_every_layout_round_trips() {
    let layouts = [
        Layout::Vertical,
        Layout::Horizontal,
        Layout::Centred,
        Layout::Scroll,
        Layout::Frame,
        Layout::Grid { columns: 4 },
        Layout::Panel {
            side: PanelSide::Top,
        },
        Layout::Panel {
            side: PanelSide::Bottom,
        },
        Layout::Panel {
            side: PanelSide::Left,
        },
        Layout::Panel {
            side: PanelSide::Right,
        },
        Layout::Panel {
            side: PanelSide::Central,
        },
    ];
    let spec = Spec {
        root: Root::Screen,
        body: Node::container(
            Layout::Vertical,
            layouts
                .into_iter()
                .enumerate()
                .map(|(i, layout)| Node::Container {
                    id: i as u32,
                    layout,
                    children: vec![],
                })
                .collect(),
        ),
    };
    assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
}

#[test]
fn a_custom_kind_travels_unchanged() {
    let spec = Spec {
        root: Root::Screen,
        body: Node::Widget {
            kind: "acme/gauge".into(),
            id: 9,
            props: vec![1, 2, 3],
        },
    };
    assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
}

#[test]
fn unknown_tags_are_rejected() {
    let bad_root = Writer::new().u8(9).finish();
    assert_eq!(
        Spec::decode(&bad_root),
        Err(DecodeError::InvalidTag {
            message: "ui root",
            tag: 9
        })
    );
    let bad_node = Writer::new().u8(1).u8(7).finish();
    assert_eq!(
        Spec::decode(&bad_node),
        Err(DecodeError::InvalidTag {
            message: "ui node",
            tag: 7
        })
    );
}

#[test]
fn a_tree_deeper_than_the_limit_is_rejected() {
    let mut node = Label { text: "x".into() }.node(0);
    for _ in 0..MAX_DEPTH {
        node = Node::container(Layout::Vertical, vec![node]);
    }
    let spec = Spec {
        root: Root::Screen,
        body: node,
    };
    assert_eq!(Spec::decode(&spec.encode()), Err(DecodeError::TooDeep));
}

#[test]
fn widgets_of_finds_one_kind_depth_first() {
    let spec = notes();
    let buttons = spec.body.widgets_of::<Button>();
    assert_eq!(
        buttons,
        vec![(
            2,
            Button {
                label: "Add".into()
            }
        )]
    );
    let labels: Vec<String> = spec
        .body
        .widgets_of::<Label>()
        .into_iter()
        .map(|(_, label)| label.text)
        .collect();
    assert_eq!(labels, ["existing note"]);
}

#[test]
fn clicked_round_trips() {
    let clicked = Clicked { id: 42 };
    assert_eq!(Clicked::decode(&clicked.encode()), Ok(clicked));
}

#[test]
fn changed_round_trips() {
    let changed = Changed {
        id: 7,
        text: "new text",
    };
    assert_eq!(Changed::decode(&changed.encode()), Ok(changed));
}
