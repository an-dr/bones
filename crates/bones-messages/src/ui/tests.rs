use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Writer};

#[test]
fn spec_with_every_widget_kind_round_trips() {
    let spec = Spec {
        title: "notes",
        widgets: vec![
            Widget::TextEdit {
                id: 1,
                text: "buy milk",
            },
            Widget::Button {
                id: 2,
                label: "Add",
            },
            Widget::Label {
                text: "existing note",
            },
        ],
    };
    assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
}

#[test]
fn empty_spec_round_trips() {
    let spec = Spec {
        title: "",
        widgets: vec![],
    };
    assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
}

#[test]
fn unknown_widget_tag_is_rejected() {
    let payload = Writer::new().str("t").u16(1).u8(255).finish();
    assert_eq!(
        Spec::decode(&payload),
        Err(DecodeError::InvalidTag {
            message: "ui widget",
            tag: 255
        })
    );
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
