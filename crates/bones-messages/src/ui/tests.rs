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

#[test]
fn set_a_events_round_trip() {
    let selected = Selected { id: 3, index: 2 };
    assert_eq!(Selected::decode(&selected.encode()), Ok(selected));
    let toggled = Toggled { id: 4, on: true };
    assert_eq!(Toggled::decode(&toggled.encode()), Ok(toggled));
    let value = ValueChanged { id: 5, value: 1.5 };
    assert_eq!(ValueChanged::decode(&value.encode()), Ok(value));
}

#[test]
fn icons_are_found_by_their_lucide_name() {
    assert_eq!(icons::by_name("trash-2"), Some(icons::TRASH_2));
    assert_eq!(icons::by_name("folder-open"), Some(icons::FOLDER_OPEN));
    assert_eq!(icons::by_name("no-such-icon"), None);
    assert!(icons::TRASH_2
        .chars()
        .all(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)));
}

#[test]
fn image_messages_round_trip() {
    let set = SetImage {
        handle: 2,
        width: 1,
        height: 1,
        rgba: &[1, 2, 3, 4],
    };
    assert_eq!(SetImage::decode(&set.encode()), Ok(set));
    let free = FreeImage { handle: 2 };
    assert_eq!(FreeImage::decode(&free.encode()), Ok(free));
}

#[test]
fn every_event_publishes_as_its_own_topic() {
    let topics: Vec<&str> = [
        Event::Clicked { id: 1 },
        Event::Changed {
            id: 1,
            text: "x".into(),
        },
        Event::Selected { id: 1, index: 0 },
        Event::Toggled { id: 1, on: true },
        Event::ValueChanged { id: 1, value: 0.0 },
    ]
    .iter()
    .map(|event| event.to_message().0)
    .collect();
    assert_eq!(
        topics,
        [
            "ui/clicked",
            "ui/changed",
            "ui/selected",
            "ui/toggled",
            "ui/value"
        ]
    );
}
