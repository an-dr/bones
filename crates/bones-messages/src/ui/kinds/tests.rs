use super::*;
use crate::ui::WidgetProps;
use crate::DecodeError;

#[test]
fn standard_kinds_round_trip() {
    let label = Label { text: "hi".into() };
    assert_eq!(Label::decode(&label.encode()), Ok(label));
    let button = Button {
        label: "Add".into(),
    };
    assert_eq!(Button::decode(&button.encode()), Ok(button));
    let edit = TextEdit {
        text: "milk".into(),
    };
    assert_eq!(TextEdit::decode(&edit.encode()), Ok(edit));
}

#[test]
fn kind_names_are_distinct() {
    let names = [Label::KIND, Button::KIND, TextEdit::KIND];
    for (i, a) in names.iter().enumerate() {
        assert!(!names[i + 1..].contains(a), "{a} is used twice");
    }
}

#[test]
fn trailing_property_bytes_are_rejected() {
    let mut bytes = Label { text: "x".into() }.encode();
    bytes.push(0);
    assert_eq!(Label::decode(&bytes), Err(DecodeError::TrailingBytes));
}
