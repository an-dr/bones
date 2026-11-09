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
    let names = [
        Label::KIND,
        Button::KIND,
        TextEdit::KIND,
        Heading::KIND,
        Weak::KIND,
        Strong::KIND,
        Selectable::KIND,
        Select::KIND,
        Checkbox::KIND,
        Slider::KIND,
        Separator::KIND,
        Hyperlink::KIND,
        Spinner::KIND,
        Space::KIND,
        Image::KIND,
        Icon::KIND,
        EmptyState::KIND,
        Progress::KIND,
        IconButton::KIND,
        Toast::KIND,
    ];
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

fn round_trips<P: WidgetProps + PartialEq + std::fmt::Debug>(props: P) {
    assert_eq!(P::decode(&props.encode()), Ok(props));
}

#[test]
fn set_a_kinds_round_trip() {
    round_trips(Heading {
        text: "Library".into(),
    });
    round_trips(Weak { text: "12".into() });
    round_trips(Strong { text: "Ada".into() });
    round_trips(Selectable {
        label: "Groups".into(),
        selected: true,
    });
    round_trips(Checkbox {
        label: "Log".into(),
        checked: false,
    });
    round_trips(Slider {
        value: 1.25,
        min: 0.5,
        max: 2.0,
    });
    round_trips(Separator);
    round_trips(Hyperlink {
        label: "Unsplash".into(),
        url: "https://unsplash.com".into(),
    });
    round_trips(Spinner);
    round_trips(Space { amount: 8.0 });
    round_trips(EmptyState {
        icon: "image".into(),
        title: "No wallpapers".into(),
        detail: "Browse to add some".into(),
    });
    round_trips(Progress {
        fraction: -1.0,
        text: "".into(),
    });
    round_trips(IconButton {
        icon: "download".into(),
        label: "Save".into(),
        primary: true,
    });
    round_trips(Toast {
        text: "Saved".into(),
    });
    round_trips(Icon {
        name: "trash-2".into(),
        size: 0.0,
    });
    for fit in [Fit::Fill, Fit::Cover, Fit::Contain] {
        round_trips(Image {
            handle: 7,
            width: 120.0,
            height: 0.0,
            fit,
            corner: 6.0,
        });
    }
}

#[test]
fn a_select_keeps_its_options_and_choice() {
    round_trips(Select {
        selected: Some(1),
        placeholder: "Source".into(),
        options: vec!["Unsplash".into(), "Pexels".into()],
    });
    round_trips(Select {
        selected: None,
        placeholder: "Any".into(),
        options: vec![],
    });
}
